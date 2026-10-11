!macro NSIS_HOOK_POSTINSTALL
  ; Persist the selected NSIS language for SeekDash's first launch.
  ; Existing in-app settings always take precedence over this value.
  StrCpy $R9 "en"
  ${If} $LANGUAGE == ${LANG_TURKISH}
    StrCpy $R9 "tr"
  ${ElseIf} $LANGUAGE == ${LANG_GERMAN}
    StrCpy $R9 "de"
  ${ElseIf} $LANGUAGE == ${LANG_FRENCH}
    StrCpy $R9 "fr"
  ${EndIf}
  WriteRegStr HKCU "Software\SeekDash\SeekDash" "InitialLanguage" "$R9"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Remove installer-only metadata. User settings and databases live under
  ; the stable com.glass.launcher app-data identity and remain untouched.
  DeleteRegValue HKCU "Software\SeekDash\SeekDash" "InitialLanguage"
  DeleteRegValue HKCU "Software\SeekDash\SeekDash" "Installer Language"
  DeleteRegValue HKCU "Software\SeekDash\SeekDash" ""
  DeleteRegKey /ifempty HKCU "Software\SeekDash\SeekDash"
  DeleteRegKey /ifempty HKCU "Software\SeekDash"
!macroend
