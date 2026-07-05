use std::time::Duration;

use smearor_http_model::HttpRequestMessage;
use smearor_http_model::HttpResponseMessage;
use smearor_http_model::HttpResponseStatus;
use smearor_swipe_launcher_plugin_api::FfiCoreContext;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use smearor_swipe_launcher_plugin_api::PluginConfig;
use smearor_swipe_launcher_plugin_api::PluginConstructionError;
use smearor_swipe_launcher_plugin_api::PluginConstructionErrorWrapper;
use smearor_swipe_launcher_plugin_api::PluginMeta;
use smearor_swipe_launcher_plugin_api::PluginMetaGetter;
use smearor_swipe_launcher_plugin_api::Service;
use smearor_swipe_launcher_plugin_api::generate_type_id;
use tracing::debug;
use tracing::error;

use crate::config::HttpServiceConfig;
use crate::whitelist::is_url_allowed;

pub struct HttpService {
    pub meta: PluginMeta,
    pub core_context: Option<FfiCoreContext>,
    pub config: HttpServiceConfig,
    pub request_sender: tokio::sync::mpsc::UnboundedSender<HttpRequestMessage>,
}

impl HttpService {
    pub(crate) fn new(config: PluginConfig, core_context: Option<FfiCoreContext>) -> Result<Self, PluginConstructionErrorWrapper> {
        let service_config: HttpServiceConfig = serde_json::from_value(config.config.clone())
            .map_err(|error| PluginConstructionErrorWrapper::new(PluginConstructionError::FailedToParseWidgetConfig, error.to_string().into()))?;

        let (request_sender, request_receiver) = tokio::sync::mpsc::unbounded_channel::<HttpRequestMessage>();
        let meta = PluginMeta::try_from(&config)?;
        let meta_clone = meta.clone();
        let core_context_clone = core_context.clone();
        let service_config_clone = service_config.clone();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    error!("HTTP service: failed to create tokio runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                run_request_handler(service_config_clone, request_receiver, meta_clone, core_context_clone).await;
            });
        });

        Ok(HttpService {
            meta,
            core_context,
            config: service_config,
            request_sender,
        })
    }
}

impl MessageHandler<String> for HttpService {
    fn handle_message(&self, message: String, _sender_id: &str) {
        match serde_json::from_str::<HttpRequestMessage>(&message) {
            Ok(request) => {
                let _ = self.request_sender.send(request);
            }
            Err(error) => {
                error!("HTTP service: failed to parse request JSON: {error}");
                error!("HTTP service: request payload was: {message}");
            }
        }
    }
}

impl MessageBroadcaster for HttpService {}

impl PluginMetaGetter for HttpService {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }
}

impl AsRef<Option<FfiCoreContext>> for HttpService {
    fn as_ref(&self) -> &Option<FfiCoreContext> {
        &self.core_context
    }
}

impl Service for HttpService {
    fn on_message(&mut self, message: *mut core::ffi::c_void) {
        if message.is_null() {
            return;
        }
        unsafe {
            let envelope = &*(message as *mut FfiEnvelope);
            if envelope.type_id == generate_type_id("std::string::String") && envelope.topic.to_string() == smearor_http_model::TOPIC_HTTP_REQUEST {
                MessageHandler::<String>::handle_envelope_message(self, envelope);
            }
        }
    }
}

async fn run_request_handler(
    config: HttpServiceConfig,
    mut request_receiver: tokio::sync::mpsc::UnboundedReceiver<HttpRequestMessage>,
    meta: PluginMeta,
    core_context: Option<FfiCoreContext>,
) {
    while let Some(request) = request_receiver.recv().await {
        debug!("HTTP service: received request for URL {}", request.url);
        if !is_url_allowed(&request.url, &config.allowed_urls) {
            let response = HttpResponseMessage {
                status: HttpResponseStatus::Forbidden,
                status_code: None,
                body: String::new(),
                error_message: Some(format!("URL is not whitelisted: {}", request.url)),
                url: request.url,
            };
            debug!("HTTP service: URL not whitelisted, returning Forbidden response");
            broadcast_response_string(&meta, &core_context, &request.response_topic, response);
            continue;
        }

        let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(config.default_timeout_ms));
        let response = execute_http_request(&request, timeout, config.max_response_bytes).await;

        match &response.status {
            HttpResponseStatus::Success => debug!("HTTP service: request succeeded with status code {:?}", response.status_code),
            HttpResponseStatus::Forbidden => debug!("HTTP service: request forbidden: {:?}", response.error_message),
            HttpResponseStatus::Error => error!("HTTP service: request failed: {:?}", response.error_message),
        }

        broadcast_response_string(&meta, &core_context, &request.response_topic, response);
    }
}

async fn execute_http_request(request: &HttpRequestMessage, timeout: Duration, max_response_bytes: usize) -> HttpResponseMessage {
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(client) => client,
        Err(error) => {
            return HttpResponseMessage {
                status: HttpResponseStatus::Error,
                status_code: None,
                body: String::new(),
                error_message: Some(format!("Failed to build HTTP client: {error}")),
                url: request.url.clone(),
            };
        }
    };

    let mut builder = match request.method {
        smearor_http_model::HttpMethod::Get => client.get(&request.url),
        smearor_http_model::HttpMethod::Post => client.post(&request.url),
        smearor_http_model::HttpMethod::Put => client.put(&request.url),
        smearor_http_model::HttpMethod::Delete => client.delete(&request.url),
    };

    if let Some(headers) = &request.headers {
        for header in headers {
            builder = builder.header(&header.name, &header.value);
        }
    }

    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }

    match builder.send().await {
        Ok(http_response) => {
            let status_code = http_response.status().as_u16();
            let body = match http_response.bytes().await {
                Ok(bytes) => {
                    let truncated = bytes.iter().take(max_response_bytes).copied().collect::<Vec<u8>>();
                    String::from_utf8_lossy(&truncated).to_string()
                }
                Err(error) => {
                    return HttpResponseMessage {
                        status: HttpResponseStatus::Error,
                        status_code: Some(status_code),
                        body: String::new(),
                        error_message: Some(format!("Failed to read response body: {error}")),
                        url: request.url.clone(),
                    };
                }
            };

            HttpResponseMessage {
                status: HttpResponseStatus::Success,
                status_code: Some(status_code),
                body,
                error_message: None,
                url: request.url.clone(),
            }
        }
        Err(error) => HttpResponseMessage {
            status: HttpResponseStatus::Error,
            status_code: None,
            body: String::new(),
            error_message: Some(format!("HTTP request failed: {error}")),
            url: request.url.clone(),
        },
    }
}

fn broadcast_response_string(meta: &PluginMeta, core_context: &Option<FfiCoreContext>, response_topic: &str, response: HttpResponseMessage) {
    let payload_str = match serde_json::to_string(&response) {
        Ok(string) => string,
        Err(error) => {
            error!("HTTP service: failed to serialize response: {error}");
            return;
        }
    };

    let payload_ptr = Box::into_raw(Box::new(payload_str)) as *mut core::ffi::c_void;
    let envelope = FfiEnvelope {
        sender_id: stabby::string::String::from(meta.id.clone()),
        target_instance_id: stabby::string::String::from(""),
        topic: stabby::string::String::from(response_topic),
        type_id: generate_type_id("std::string::String"),
        payload: payload_ptr,
        destroy_payload: Some(destroy_payload_string),
        clone_payload: Some(clone_payload_string),
    };

    if let Some(context) = core_context {
        context.send_message(envelope);
        debug!("HTTP service: broadcasted response on topic {}", response_topic);
    }
}

extern "C" fn clone_payload_string(ptr: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let value = unsafe { &*(ptr as *const String) };
    Box::into_raw(Box::new(value.clone())) as *mut core::ffi::c_void
}

extern "C" fn destroy_payload_string(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut String);
        }
    }
}
