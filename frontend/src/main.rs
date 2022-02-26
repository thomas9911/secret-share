#![recursion_limit = "1024"]

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use crypto_part::{decode, encode, random_bytes, sha256, Key};
use std::io::Cursor;

use console_error_panic_hook::set_once as set_panic_hook;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::window;
use web_sys::{
    Document, Element, HtmlAnchorElement, HtmlButtonElement, HtmlElement, HtmlTextAreaElement,
    Window,
};

mod http;
pub use http::{APIRequest, FakeBackend, RealBackend};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

pub(crate) use console_log;

fn base64(input: &[u8]) -> String {
    base64::encode_config(input, base64::URL_SAFE)
}

fn base64_decode(input: &[u8]) -> Result<Vec<u8>, base64::DecodeError> {
    base64::decode_config(input, base64::URL_SAFE)
}

struct MainDocumentBuilder<R>
where
    R: APIRequest,
{
    http_request: R,
}

impl<R: APIRequest + 'static> MainDocumentBuilder<R> {
    fn new(http_request: R) -> Self {
        MainDocumentBuilder { http_request }
    }

    fn on_document(&self, document: Document) -> Result<(), JsValue> {
        let body = document.body().expect("Could not access document.body");

        let title = Self::create_title(&document)?;
        let text_area = Self::create_text_area(&document)?;
        let button = self.create_button(document)?;

        body.append_child(title.as_ref())?;
        body.append_child(text_area.as_ref())?;
        body.append_child(button.as_ref())?;

        Ok(())
    }

    fn create_title(document: &Document) -> Result<Element, JsValue> {
        let title = document.create_element("h1")?;
        title.set_text_content(Some("hello"));
        title.set_id("title");
        Ok(title)
    }

    fn create_text_area(document: &Document) -> Result<Element, JsValue> {
        let text_area = document.create_element("textarea")?;
        let text_area_ref = text_area
            .dyn_ref::<HtmlTextAreaElement>()
            .expect("it is a textarea");
        text_area_ref.set_name("textarea");
        text_area_ref.set_rows(24);
        text_area_ref.set_cols(80);

        Ok(text_area)
    }

    fn create_button(&self, document: Document) -> Result<Element, JsValue> {
        let button = document.create_element("button")?;
        button.set_text_content(Some("Submit"));
        let button_ref = button
            .dyn_ref::<HtmlButtonElement>()
            .expect("button is an input");
        button_ref.set_name("button");
        self.attach_action_to_button(document, button_ref);
        Ok(button)
    }

    fn attach_action_to_button(&self, document: Document, button: &HtmlButtonElement) {
        let http_request = self.http_request.clone();
        let func = Box::new(move || Self::button_action(document.clone(), http_request.clone()));
        let closure = Closure::wrap(func as Box<dyn Fn()>);
        button.set_onclick(Some(closure.as_ref().unchecked_ref()));
        // let Rust forget about this
        closure.forget();
    }

    fn button_action(document: Document, http_request: R) {
        let textarea = document
            .get_elements_by_name("textarea")
            .get(0)
            .expect("Could not find textarea");

        let textarea_ref = textarea
            .dyn_ref::<HtmlTextAreaElement>()
            .expect("it is a textarea");

        let mut out_bytes = Vec::new();
        let mut recoded_bytes = Vec::new();

        let generated_secret = base64(&random_bytes());
        console_log!("{:?}", generated_secret);
        let key = Key::from(&generated_secret);

        let input_text = textarea_ref.value();
        let in_bytes = Cursor::new(&input_text);
        encode(in_bytes, &mut out_bytes, &key).unwrap_throw();
        decode(&out_bytes[..], &mut recoded_bytes, &key).unwrap_throw();

        // just verify that the input and output are the same
        assert_eq!(input_text, String::from_utf8(recoded_bytes).unwrap_throw());

        let hashed = base64(&out_bytes);
        let id = base64(&sha256(&hashed));

        console_log!("{:?}", hashed);
        let window = window().expect("Could not access window");
        let base_url = window.location().href().unwrap_throw();

        let url = format!("{}?reload=true#{}", base_url, hashed);
        http_request
            .post(id, generated_secret, move |_| {
                let out = get_or_create_element(&document, "a", "output_url");
                out.set_text_content(Some(&url));
                out.set_attribute("href", &url)?;

                let body = document.body().unwrap_throw();
                body.append_child(out.as_ref())?;
                Ok(JsValue::NULL)
            })
            .unwrap_throw();
    }
}

struct ResultDocumentBuilder<'a, R>
where
    R: APIRequest,
{
    http_request: R,
    id: &'a str,
    payload: &'a str,
}

impl<'a, R: APIRequest> ResultDocumentBuilder<'a, R> {
    fn new(http_request: R, id: &'a str, payload: &'a str) -> ResultDocumentBuilder<'a, R> {
        ResultDocumentBuilder {
            http_request,
            id,
            payload,
        }
    }

    fn on_document(&self, document: &Document) -> Result<(), JsValue> {
        let body = document.body().expect("Could not access document.body");

        let text = get_or_create_element(document, "pre", "output");
        let link = Self::create_link_to_main(document)?;

        body.append_child(link.as_ref())?;
        body.append_child(text.as_ref())?;

        Ok(())
    }

    fn decode(&self, document: &Document) -> Result<(), JsValue> {
        let document = document.clone();
        let payload = base64_decode(self.payload.as_bytes()).expect("cannot parse payload");
        self.http_request
            .get(self.id.to_string(), move |response| {
                let secret = response
                    .as_string()
                    .ok_or(JsValue::from_str("cannot parse request"))?;

                let output = get_or_create_element(&document, "pre", "output");
                if let Err(_) = base64_decode(secret.as_bytes()) {
                    output.set_text_content(Some(&secret));
                } else {
                    let key = Key::from(secret);
                    let in_bytes = Cursor::new(&payload);
                    let mut out_bytes = Vec::new();

                    decode(in_bytes, &mut out_bytes, &key).unwrap_throw();

                    output.set_text_content(Some(
                        &String::from_utf8(out_bytes).expect("bytes not utf8"),
                    ));
                };

                Ok(JsValue::NULL)
            })?;

        Ok(())
    }

    fn create_link_to_main(document: &Document) -> Result<Element, JsValue> {
        let link = get_or_create_element(document, "a", "link_to_main");

        let link_ref = link.dyn_ref::<HtmlAnchorElement>().expect("it is a anchor");
        link_ref.set_href("/");
        link_ref.set_text("go back to index")?;

        Ok(link)
    }
}

fn get_or_create_element(document: &Document, tag: &str, id: &str) -> Element {
    document.get_element_by_id(id).unwrap_or_else(|| {
        let element = document.create_element(tag).unwrap_throw();
        element.set_id(id);
        element
    })
}

fn start_app() -> Result<(), JsValue> {
    let window = window().expect("Could not access window");
    let document = window.document().expect("Could not access document");

    let location = window.location();
    let mut location_hash = location.hash().expect("Could not access hash");

    let backend = RealBackend;

    if location_hash.len() == 0 {
        let builder = MainDocumentBuilder::new(backend);
        builder.on_document(document)?;
    } else {
        location_hash.remove(0);
        let payload = location_hash.as_ref();
        let id = base64(&sha256(payload));

        let builder = ResultDocumentBuilder::new(backend, &id, payload);
        builder.on_document(&document)?;
        builder.decode(&document)?;
    }

    Ok(())
}

fn main() -> Result<(), JsValue> {
    set_panic_hook();
    start_app()?;

    Ok(())
}

// use spawn_local function to do the fetching part
