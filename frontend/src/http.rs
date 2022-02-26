use crate::console_log;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};

pub trait APIRequest: Clone {
    fn post<C: Fn(Response) -> Result<JsValue, JsValue> + 'static>(
        &self,
        id: String,
        secret: String,
        cb: C,
    ) -> Result<JsValue, JsValue>;

    fn get<C: Fn(JsValue) -> Result<JsValue, JsValue> + 'static>(
        &self,
        id: String,
        cb: C,
    ) -> Result<JsValue, JsValue>;
}

#[derive(Debug, Clone)]
pub struct FakeBackend;

impl APIRequest for FakeBackend {
    fn post<C: Fn(Response) -> Result<JsValue, JsValue> + 'static>(
        &self,
        _id: String,
        _secret: String,
        cb: C,
    ) -> Result<JsValue, JsValue> {
        cb(Response::new()?)
    }

    fn get<C: Fn(JsValue) -> Result<JsValue, JsValue> + 'static>(
        &self,
        _id: String,
        cb: C,
    ) -> Result<JsValue, JsValue> {
        cb(JsValue::NULL)?;

        Ok(JsValue::NULL)
    }
}

#[derive(Debug, Clone)]
pub struct RealBackend;

impl APIRequest for RealBackend {
    fn post<C: Fn(Response) -> Result<JsValue, JsValue> + 'static>(
        &self,
        id: String,
        secret: String,
        cb: C,
    ) -> Result<JsValue, JsValue> {
        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.mode(RequestMode::Cors);
        opts.body(Some(&JsValue::from(secret)));

        let url = format!("/api/{}", id);

        let request = Request::new_with_str_and_init(&url, &opts)?;

        // request.headers();

        let window = web_sys::window().expect("Window unavailable");
        let resp_value_promise = JsFuture::from(window.fetch_with_request(&request));

        spawn_local(async move {
            let resp_value = resp_value_promise.await.unwrap_throw();

            let resp: Response = resp_value.dyn_into().unwrap();
            cb(resp).unwrap();
        });

        Ok(JsValue::NULL)
    }

    fn get<C: Fn(JsValue) -> Result<JsValue, JsValue> + 'static>(
        &self,
        id: String,
        cb: C,
    ) -> Result<JsValue, JsValue> {
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);

        let url = format!("/api/{}", id);

        let request = Request::new_with_str_and_init(&url, &opts)?;

        let window = web_sys::window().expect("Window unavailable");
        let resp_value_promise = JsFuture::from(window.fetch_with_request(&request));

        spawn_local(async move {
            let resp_value = resp_value_promise.await.unwrap_throw();

            let resp: Response = resp_value.dyn_into().unwrap();
            let text = JsFuture::from(resp.text().unwrap_throw())
                .await
                .unwrap_throw();
            cb(text).unwrap();
        });

        Ok(JsValue::NULL)
    }
}
