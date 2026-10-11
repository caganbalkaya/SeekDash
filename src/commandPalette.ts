export type CommandId = "lock" | "sleep" | "restart" | "shutdown" | "recycle" | "ip" | "uuid" | "color" | "settings";

import type { TranslationKey } from "./i18n";

export interface CommandDefinition {
  id: CommandId;
  titleKey: TranslationKey;
  descriptionKey: TranslationKey;
  icon: "lock" | "moon" | "restart" | "power" | "trash" | "network" | "hash" | "palette" | "settings";
  destructive?: boolean;
}

export const COMMANDS: readonly CommandDefinition[] = [
  { id: "lock", titleKey: "command.lock.title", descriptionKey: "command.lock.desc", icon: "lock" },
  { id: "sleep", titleKey: "command.sleep.title", descriptionKey: "command.sleep.desc", icon: "moon", destructive: true },
  { id: "restart", titleKey: "command.restart.title", descriptionKey: "command.restart.desc", icon: "restart", destructive: true },
  { id: "shutdown", titleKey: "command.shutdown.title", descriptionKey: "command.shutdown.desc", icon: "power", destructive: true },
  { id: "recycle", titleKey: "command.recycle.title", descriptionKey: "command.recycle.desc", icon: "trash" },
  { id: "ip", titleKey: "command.ip.title", descriptionKey: "command.ip.desc", icon: "network" },
  { id: "uuid", titleKey: "command.uuid.title", descriptionKey: "command.uuid.desc", icon: "hash" },
  { id: "color", titleKey: "command.color.title", descriptionKey: "command.color.desc", icon: "palette" },
  { id: "settings", titleKey: "command.settings.title", descriptionKey: "command.settings.desc", icon: "settings" },
];

export function commandArgument(query: string): string {
  return query.slice(1).trim().split(/\s+/, 2)[1] ?? "";
}

export function filterCommands(query: string): CommandDefinition[] {
  if (!query.startsWith(">")) return [];
  const token = query.slice(1).trimStart().split(/\s+/, 1)[0].toLocaleLowerCase("en-US");
  if (!token) return [...COMMANDS];
  return COMMANDS.filter((command) => command.id.startsWith(token));
}

export function hexToRgb(value: string): string | null {
  const match = /^#?([\da-f]{3}|[\da-f]{6})$/i.exec(value.trim());
  if (!match) return null;
  const hex = match[1].length === 3 ? [...match[1]].map((part) => part + part).join("") : match[1];
  const number = Number.parseInt(hex, 16);
  return `rgb(${number >> 16}, ${(number >> 8) & 255}, ${number & 255})`;
}
