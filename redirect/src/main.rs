//! Wat er van deze repository overblijft: een permanente verwijzing naar de
//! poc achter het portaal.
//!
//! De Napp-poc draaide op `napp-poc.rijks.app` uit deze repo. Hij leeft nu in
//! de monorepo (MinBZK/regelrecht) en hangt onder het poc-portaal op `/napp/`.
//! Dit adres blijft bestaan zolang er links naar rondgaan, maar het serveert
//! de casus niet meer: het portaal zet er een wachtwoord voor, en dit adres
//! liep daaromheen.
//!
//! Alles wat hier stond is naar de monorepo verhuisd; deze repo is daarna
//! gearchiveerd. Er valt hier dus niets meer te bouwen of te wijzigen.
//!
//! 301 en niet 302: het adres is opgeheven, niet tijdelijk verplaatst. Een
//! browser die dat eenmaal weet, vraagt het oude adres niet opnieuw op.
//!
//! Het pad blijft behouden, want beide kanten kennen dezelfde ingangen:
//! `/aanvrager/` wordt `/napp/aanvrager/`. Een doorgestuurde diepe link komt
//! zo op dezelfde pagina uit in plaats van op de voordeur.

use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;

/// Waar het heen gaat. Zonder afsluitende slash: het pad dat we eraan plakken
/// begint er zelf met een.
const DOEL: &str = "https://poc.regelrecht.rijks.app/napp";

/// De poort waarop ZAD dit component aanspreekt.
const STANDAARD_POORT: u16 = 8000;

/// De poort uit `PORT`, of de standaard. Een onleesbare waarde valt terug op
/// de standaard in plaats van de container te laten stoppen: dit ding heeft
/// maar één taak en die moet het onder alle omstandigheden doen.
fn poort() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(STANDAARD_POORT)
}

/// Bouwt de doel-URL uit het binnenkomende pad.
///
/// Alleen `path_and_query` van het verzoek gaat mee, nooit een host uit een
/// header. Een verzoek komt hier binnen als een absoluut pad, dus er valt geen
/// `//elders.example` in te schuiven: dat zou vanaf de tweede slash als andere
/// host gelezen worden, en dan stuurt dit adres bezoekers naar een site die
/// wij niet zijn.
fn doel_url(pad_en_query: &str) -> String {
    let pad = pad_en_query.strip_prefix('/').unwrap_or(pad_en_query);
    // Elke leidende slash eraf: `//elders` en `/\elders` zijn allebei een
    // protocol-relatieve URL zodra ze achter `https://host` belanden.
    let pad = pad.trim_start_matches(['/', '\\']);
    if pad.is_empty() {
        format!("{DOEL}/")
    } else {
        format!("{DOEL}/{pad}")
    }
}

async fn verwijs(request: Request) -> Response {
    let pad = request
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");
    match HeaderValue::from_str(&doel_url(pad)) {
        Ok(locatie) => {
            (StatusCode::MOVED_PERMANENTLY, [(header::LOCATION, locatie)]).into_response()
        }
        // Een pad dat niet in een header past, is geen pad dat wij kennen.
        Err(_) => (
            StatusCode::MOVED_PERMANENTLY,
            [(header::LOCATION, HeaderValue::from_static(DOEL))],
        )
            .into_response(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().fallback(verwijs);
    let poort = poort();
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", poort)).await?;
    println!("napp-redirect luistert op :{poort} en verwijst naar {DOEL}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    async fn locatie_van(pad: &str) -> (StatusCode, String) {
        let app = Router::new().fallback(verwijs);
        let response = app
            .oneshot(HttpRequest::builder().uri(pad).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let locatie = response
            .headers()
            .get(header::LOCATION)
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        (status, locatie)
    }

    #[tokio::test]
    async fn de_wortel_gaat_naar_de_poc() {
        let (status, locatie) = locatie_van("/").await;
        assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
        assert_eq!(locatie, "https://poc.regelrecht.rijks.app/napp/");
    }

    #[tokio::test]
    async fn een_diepe_link_houdt_zijn_pad() {
        // De reden dat het pad meegaat: beide kanten kennen deze ingang.
        let (_, locatie) = locatie_van("/aanvrager/").await;
        assert_eq!(locatie, "https://poc.regelrecht.rijks.app/napp/aanvrager/");
    }

    #[tokio::test]
    async fn de_query_gaat_mee() {
        let (_, locatie) = locatie_van("/register?zoek=test").await;
        assert_eq!(
            locatie,
            "https://poc.regelrecht.rijks.app/napp/register?zoek=test"
        );
    }

    #[tokio::test]
    async fn een_dubbele_slash_wijst_niet_naar_een_andere_host() {
        // `//elders.example` achter `https://poc…/napp` zou vanaf die twee
        // slashes als eigen host gelezen worden. Dan stuurt dit adres
        // bezoekers naar een site die wij niet zijn.
        let (_, locatie) = locatie_van("//elders.example/pad").await;
        assert_eq!(
            locatie,
            "https://poc.regelrecht.rijks.app/napp/elders.example/pad"
        );
    }

    #[tokio::test]
    async fn een_backslash_telt_net_zo_goed_als_een_slash() {
        // Browsers lezen `/\elders` als protocol-relatief, net als `//elders`.
        let (_, locatie) = locatie_van("/\\elders.example/pad").await;
        assert_eq!(
            locatie,
            "https://poc.regelrecht.rijks.app/napp/elders.example/pad"
        );
    }

    #[tokio::test]
    async fn elke_methode_verwijst() {
        // Ook een POST hoort het nieuwe adres te horen, niet een 405.
        let app = Router::new().fallback(verwijs);
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/aanvragen")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }
}
