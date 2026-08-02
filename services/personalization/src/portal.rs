use serde::Deserialize;
use smearor_personalization_model::ColorScheme;
use smearor_personalization_model::GeoCoordinates;
use tracing::debug;
use tracing::warn;

/// Photon reverse geocoding API response structure.
#[derive(Debug, Deserialize)]
struct PhotonResponse {
    features: Vec<PhotonFeature>,
}

#[derive(Debug, Deserialize)]
struct PhotonFeature {
    properties: PhotonProperties,
}

#[derive(Debug, Default, Deserialize)]
struct PhotonProperties {
    name: Option<String>,
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
}

/// Derives a human-readable location name from Photon properties.
///
/// Preference order: city > name > state > country.
fn format_location_name(props: &PhotonProperties) -> Option<String> {
    if let Some(city) = &props.city {
        return Some(city.clone());
    }
    if let Some(name) = &props.name {
        return Some(name.clone());
    }
    if let Some(state) = &props.state {
        return Some(state.clone());
    }
    if let Some(country) = &props.country {
        return Some(country.clone());
    }
    None
}

/// Reverse geocodes coordinates to a location name via the Photon API (photon.komoot.io).
///
/// Returns `None` if the API is unavailable or no result is found.
pub async fn reverse_geocode(latitude: f64, longitude: f64) -> Option<String> {
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build() {
        Ok(client) => client,
        Err(error) => {
            warn!("personalization: failed to create HTTP client for reverse geocoding: {error}");
            return None;
        }
    };

    let url = format!("https://photon.komoot.io/reverse?lat={}&lon={}&limit=1", latitude, longitude);
    let response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!("personalization: reverse geocoding request failed: {error}");
            return None;
        }
    };

    let photon: PhotonResponse = match response.json().await {
        Ok(data) => data,
        Err(error) => {
            warn!("personalization: failed to parse reverse geocoding response: {error}");
            return None;
        }
    };

    let name = photon.features.first().and_then(|f| format_location_name(&f.properties));
    if let Some(ref name) = name {
        debug!("personalization: reverse geocoding succeeded for ({}, {}): {}", latitude, longitude, name);
    } else {
        debug!("personalization: reverse geocoding returned no results for ({}, {})", latitude, longitude);
    }
    name
}

/// Queries the user's GPS location via the XDG Desktop Portal.
///
/// Creates a one-shot location session with street-level accuracy,
/// waits for the first location update, then closes the session.
/// Returns `None` if the portal is unavailable or the user denies access.
pub async fn query_location() -> Option<GeoCoordinates> {
    let proxy = match ashpd::desktop::location::LocationProxy::new().await {
        Ok(proxy) => proxy,
        Err(error) => {
            warn!("personalization: failed to create LocationProxy: {error}");
            return None;
        }
    };

    let session = match proxy
        .create_session(ashpd::desktop::location::CreateSessionOptions::default().set_accuracy(ashpd::desktop::location::Accuracy::Street))
        .await
    {
        Ok(session) => session,
        Err(error) => {
            warn!("personalization: failed to create location session: {error}");
            return None;
        }
    };

    let proxy_clone = &proxy;
    let session_clone = &session;
    let start_fut = proxy_clone.start(session_clone, None, Default::default());
    let stream_fut = proxy_clone.receive_location_updated();

    let (start_result, stream_result) = futures_util::future::join(start_fut, stream_fut).await;

    if let Err(error) = start_result {
        warn!("personalization: failed to start location session: {error}");
        let _ = session.close().await;
        return None;
    }

    let mut stream = match stream_result {
        Ok(stream) => stream,
        Err(error) => {
            warn!("personalization: failed to receive location updates: {error}");
            let _ = session.close().await;
            return None;
        }
    };

    let location = match futures_util::StreamExt::next(&mut stream).await {
        Some(location) => location,
        None => {
            debug!("personalization: location stream exhausted without data");
            let _ = session.close().await;
            return None;
        }
    };

    let coords = GeoCoordinates {
        latitude: location.latitude(),
        longitude: location.longitude(),
        location_name: stabby::option::Option::None(),
    };

    let _ = session.close().await;
    debug!("personalization: location query succeeded: lat={}, lon={}", coords.latitude, coords.longitude);
    Some(coords)
}

/// Queries the system's preferred color scheme via the XDG Desktop Portal.
///
/// Returns `None` if the portal is unavailable.
pub async fn query_color_scheme() -> Option<ColorScheme> {
    let settings = match ashpd::desktop::settings::Settings::new().await {
        Ok(settings) => settings,
        Err(error) => {
            warn!("personalization: failed to create Settings proxy: {error}");
            return None;
        }
    };

    match settings.color_scheme().await {
        Ok(scheme) => {
            let result = match scheme {
                ashpd::desktop::settings::ColorScheme::NoPreference => ColorScheme::System,
                ashpd::desktop::settings::ColorScheme::PreferDark => ColorScheme::Dark,
                ashpd::desktop::settings::ColorScheme::PreferLight => ColorScheme::Light,
            };
            debug!("personalization: color scheme query succeeded: {:?}", result);
            Some(result)
        }
        Err(error) => {
            warn!("personalization: failed to read color scheme: {error}");
            None
        }
    }
}
