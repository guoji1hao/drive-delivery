import { ref, watch } from "vue";
import { en } from "./en";
import { guideEnglish } from "./guides";

export type Locale = "zh-CN" | "en";
function initialLocale(): Locale {
  try { return localStorage.getItem("drive-delivery-language") === "en" ? "en" : "zh-CN"; } catch { return "zh-CN"; }
}
export const locale = ref<Locale>(initialLocale());
watch(locale, value => {
  if (typeof document !== "undefined") { document.documentElement.lang = value; document.title = value === "en" ? "Drive Delivery" : "双盘发货助手"; }
  try { localStorage.setItem("drive-delivery-language", value); } catch { /* Language still works for this session. */ }
}, { immediate: true });

export function t(text: unknown): string {
  const value = String(text ?? "");
  if (locale.value !== "en") return value;
  const translated = en[value] ?? guideEnglish[value];
  if (translated) return translated;
  const prefixes: Record<string, string> = { "配置错误：": "Configuration: ", "认证失败：": "Sign-in failed: ", "文件处理失败：": "File error: ", "网盘接口失败：": "Drive API error: ", "本地数据库错误：": "Local database error: ", "系统安全存储错误：": "Secure storage error: " };
  for (const [prefix, english] of Object.entries(prefixes)) {
    if (value.startsWith(prefix)) return english + t(value.slice(prefix.length));
  }
  return value;
}
export function tf(zh: string, english: string, values: Record<string, string | number>): string {
  return (locale.value === "en" ? english : zh).replace(/\{(\w+)\}/g, (match, key) => String(values[key] ?? match));
}
