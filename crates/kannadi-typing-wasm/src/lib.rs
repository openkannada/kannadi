//! WebAssembly bindings for kannadi-typing.
//!
//! The model is not compiled in: the page fetches `kn.typing` and passes its
//! bytes to the constructor, so the WebAssembly file stays small and the model
//! can be cached on its own.
//!
//! ```js
//! import init, { TypingEngine } from "./kannadi_typing_wasm.js";
//! await init();
//! const bytes = new Uint8Array(await (await fetch("kn.typing")).arrayBuffer());
//! const engine = new TypingEngine(bytes);
//! engine.suggest("kannada", undefined, 5); // ["ಕನ್ನಡ", ...]
//! engine.learn("til", "ತಿಳಿ");
//! localStorage.setItem("picks", engine.exportUser());
//! ```

use kannadi_typing::{Engine, Model};
use wasm_bindgen::prelude::*;

/// A typing engine: the model and 1 user's picks, held in memory.
#[wasm_bindgen]
pub struct TypingEngine {
    inner: Engine,
}

#[wasm_bindgen]
impl TypingEngine {
    /// Loads a model from the bytes of `kn.typing`. Throws if the bytes are not a model.
    #[wasm_bindgen(constructor)]
    pub fn new(model: &[u8]) -> Result<TypingEngine, JsError> {
        let model = Model::from_bytes(model).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(TypingEngine {
            inner: Engine::new(model),
        })
    }

    /// The best `n` Kannada words for the Latin letters `input`, best first.
    /// `prev` is the Kannada word before the cursor, if any. The first word is
    /// what the letters spell; later ones may be longer words they start.
    /// Input with anything other than ASCII letters gives an empty list.
    pub fn suggest(&self, input: &str, prev: Option<String>, n: usize) -> Vec<String> {
        self.inner.suggest(input, prev.as_deref(), n)
    }

    /// Records that the user chose `word` for `input`. Returns false if `word` is not Kannada.
    pub fn learn(&mut self, input: &str, word: &str) -> bool {
        self.inner.learn(input, word)
    }

    /// Forgets every pick of `word`.
    pub fn forget(&mut self, word: &str) {
        self.inner.forget(word);
    }

    /// The user's picks as text, to keep in the browser's own storage.
    #[wasm_bindgen(js_name = exportUser)]
    pub fn export_user(&self) -> String {
        self.inner.export_user()
    }

    /// Restores picks from `exportUser`. Returns false, and keeps the current picks, for other text.
    #[wasm_bindgen(js_name = importUser)]
    pub fn import_user(&mut self, text: &str) -> bool {
        self.inner.import_user(text)
    }
}
