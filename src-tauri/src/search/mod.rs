use crate::{
    models::{ResultKind, SearchResult},
    state::EngineState,
};
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

pub fn normalize(value: &str) -> String {
    value
        .nfkd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'ı' => 'i',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn execute(
    state: &EngineState,
    query: &str,
    limit: usize,
    request_id: u64,
) -> Result<Vec<SearchResult>, String> {
    #[cfg(debug_assertions)]
    let started = std::time::Instant::now();
    let needle = normalize(query.trim());
    if let Some(value) = calculator(query) {
        return Ok(vec![SearchResult {
            id: format!("calc:{query}"),
            title: value.clone(),
            subtitle: format!("i18n:search.calculatorCopy|{query}"),
            path: value,
            kind: ResultKind::Calculator,
            score: 1000.0,
            icon: None,
            favorite: false,
        }]);
    }
    if state
        .latest_request
        .load(std::sync::atomic::Ordering::Acquire)
        != request_id
    {
        return Ok(Vec::new());
    }
    let lock_started = std::time::Instant::now();
    let db = state.search_db.lock();
    let lock_wait = lock_started.elapsed();
    if state
        .latest_request
        .load(std::sync::atomic::Ordering::Acquire)
        != request_id
    {
        return Ok(Vec::new());
    }
    if lock_wait.as_millis() > 50 {
        eprintln!("SeekDash slow-operation: search DB lock waited {lock_wait:?}");
    }
    let mut results = db.search_candidates(&needle, limit).unwrap_or_default();
    if !needle.is_empty() && results.len() < 8 {
        results.extend(
            db.fuzzy_candidates(&needle, limit)
                .map_err(|e| e.to_string())?,
        );
    }
    drop(db);
    let mut seen = HashSet::new();
    results.retain(|result| seen.insert(result_identity(state, result)));
    for result in &mut results {
        let name = normalize(&result.title);
        if result.favorite {
            result.score += 10_000.0;
        }
        result.score += match_score(&needle, &name);
    }
    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.title.cmp(&b.title))
    });
    results.truncate(limit);
    if results.is_empty() && !needle.is_empty() {
        results.push(SearchResult {
            id: format!("web:{query}"),
            title: format!("i18n:search.webTitle|{}", query.trim()),
            subtitle: "i18n:search.webSubtitle".into(),
            path: query.trim().into(),
            kind: ResultKind::WebSearch,
            score: 0.0,
            icon: None,
            favorite: false,
        });
    }
    #[cfg(debug_assertions)]
    eprintln!(
        "SeekDash search: {} results in {:.2}ms",
        results.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    Ok(results)
}

fn path_identity(path: &str) -> String {
    let normalized = path.replace('/', "\\");
    normalized
        .strip_prefix(r"\\?\")
        .unwrap_or(&normalized)
        .trim_end_matches('\\')
        .to_lowercase()
}

fn result_identity(state: &EngineState, result: &SearchResult) -> String {
    if result.path.to_ascii_lowercase().ends_with(".lnk") {
        if let Some(cached) = state.shortcut_cache.lock().get(&result.path).cloned() {
            return cached;
        }
        if let Some(identity) = crate::launcher::shortcut_identity(&result.path) {
            let mut cache = state.shortcut_cache.lock();
            if cache.len() >= 256 {
                if let Some(key) = cache.keys().next().cloned() {
                    cache.remove(&key);
                }
            }
            cache.insert(result.path.clone(), identity.clone());
            return identity;
        }
    }
    path_identity(&result.path)
}

fn match_score(needle: &str, name: &str) -> f64 {
    if name == needle {
        1_000_000.0
    } else if name.starts_with(needle) {
        750_000.0
    } else if name.split_whitespace().any(|word| word.starts_with(needle)) {
        600_000.0
    } else if name.contains(needle) {
        400_000.0
    } else {
        strsim::jaro_winkler(needle, name) * 100_000.0
    }
}

fn calculator(query: &str) -> Option<String> {
    let input = query.trim();
    if input.len() < 3
        || !input
            .chars()
            .all(|c| c.is_ascii_digit() || "+-*/().% ^".contains(c))
        || !input.chars().any(|c| "+-*/%^".contains(c))
    {
        return None;
    }
    let value = meval::eval_str(input).ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.8}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::{match_score, normalize, path_identity};
    #[test]
    fn normalizes_turkish_and_unicode() {
        assert_eq!(normalize("İndirilen Öğeler"), "indirilen ogeler");
        assert_eq!(normalize("IŞIK  Proje"), "isik proje");
    }
    #[test]
    fn duplicate_space_normalization() {
        assert_eq!(normalize("  My   File  "), "my file");
    }
    #[test]
    fn exact_prefix_and_fuzzy_tiers_are_stable() {
        let exact = match_score("note", "note");
        let prefix = match_score("note", "notebook");
        let fuzzy = match_score("note", "ntoe");
        assert!(exact > prefix && prefix > fuzzy);
    }
    #[test]
    fn windows_path_identity_is_case_and_prefix_insensitive() {
        assert_eq!(
            path_identity(r"\\?\C:\Apps\GLASS.exe"),
            path_identity("c:/apps/glass.exe")
        );
    }
}
