import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface TranslateResult {
  original: string;
  translated: string;
  fromLang: string;
  toLang: string;
}

export function useTranslate() {
  const isTranslating = ref(false);
  const error = ref<string | null>(null);
  const lastResult = ref<TranslateResult | null>(null);

  async function translate(
    text: string,
    fromLang: string = "en",
    toLang: string = "zh"
  ): Promise<TranslateResult | null> {
    if (!text.trim()) {
      error.value = "Empty text";
      return null;
    }

    isTranslating.value = true;
    error.value = null;

    try {
      const translated: string = await invoke("translate_text", {
        text,
        fromLang,
        toLang,
      });

      const result: TranslateResult = {
        original: text,
        translated,
        fromLang,
        toLang,
      };

      lastResult.value = result;
      return result;
    } catch (e) {
      error.value = String(e);
      return null;
    } finally {
      isTranslating.value = false;
    }
  }

  return {
    isTranslating,
    error,
    lastResult,
    translate,
  };
}