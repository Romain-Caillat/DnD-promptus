//! platform/sign-in-gm — account creation and sign-in with a passkey,
//! through the real router and real WebAuthn verification.
//!
//! The authenticator is `webauthn-authenticator-rs`'s software passkey:
//! real key pairs, real signatures over the server's challenges. Two
//! things it cannot do are done by the test "browser", both outside what
//! the signature covers (same approach as Devotion): it cannot store a
//! discoverable credential (the test clears that request and remembers
//! the credential itself), and it does not return the user handle (the
//! test adds it, as a platform authenticator would).

mod common;

use axum::Router;
use axum::http::StatusCode;
use common::{Reply, call};
use promptus_back::auth::setup::SetupState;
use promptus_back::auth::tokens::hash_token;
use serde_json::{Value, json};
use sqlx::PgPool;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse, Url};
use webauthn_rs_proto::{AllowCredentials, UserVerificationPolicy};

const SETUP_CODE: &str = "setup-code-for-the-tests-0123456789";

// ------------------------------------------------------------ the "browser"

/// One person's authenticator (a phone's Face ID, a YubiKey).
struct Authenticator {
    inner: WebauthnAuthenticator<SoftPasskey>,
    origin: Url,
    /// (credential id, user handle) of every credential it created.
    creds: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Authenticator {
    fn new() -> Self {
        Self::at(common::ORIGIN)
    }

    fn at(origin: &str) -> Self {
        // `true`: the soft key claims user verification, as Face ID does.
        Self {
            inner: WebauthnAuthenticator::new(SoftPasskey::new(true)),
            origin: Url::parse(origin).unwrap(),
            creds: Vec::new(),
        }
    }

    fn create(&mut self, options: &Value) -> Value {
        assert_no_null(options);
        let mut ccr: CreationChallengeResponse = serde_json::from_value(options.clone()).unwrap();
        let sel = ccr
            .public_key
            .authenticator_selection
            .as_mut()
            .expect("authenticator selection");
        assert!(
            sel.require_resident_key,
            "the server asks for a discoverable credential"
        );
        assert_eq!(sel.user_verification, UserVerificationPolicy::Required);
        // The soft key cannot store it; this test remembers it instead.
        sel.require_resident_key = false;
        sel.resident_key = None;
        let user: Vec<u8> = ccr.public_key.user.id.to_vec();
        let reg = self
            .inner
            .do_registration(self.origin.clone(), ccr)
            .expect("register");
        self.creds.push((reg.raw_id.to_vec(), user));
        serde_json::to_value(&reg).unwrap()
    }

    fn get(&mut self, options: &Value) -> Value {
        assert_no_null(options);
        let mut rcr: RequestChallengeResponse = serde_json::from_value(options.clone()).unwrap();
        assert_eq!(
            rcr.public_key.user_verification,
            UserVerificationPolicy::Required
        );
        assert!(
            rcr.public_key.allow_credentials.is_empty(),
            "signing in names no account"
        );
        rcr.public_key.allow_credentials = self
            .creds
            .iter()
            .map(|(id, _)| AllowCredentials {
                type_: "public-key".into(),
                id: id.clone().into(),
                transports: None,
            })
            .collect();
        let mut cred = self
            .inner
            .do_authentication(self.origin.clone(), rcr)
            .expect("assert");
        let raw = cred.raw_id.to_vec();
        let handle = self
            .creds
            .iter()
            .find(|(id, _)| *id == raw)
            .unwrap()
            .1
            .clone();
        cred.response.user_handle = Some(handle.into());
        serde_json::to_value(&cred).unwrap()
    }
}

/// Browsers reject `null` where WebAuthn expects an enum or a list (a
/// TypeError before any prompt): a challenge must never carry one.
fn assert_no_null(v: &Value) {
    match v {
        Value::Null => panic!("a WebAuthn challenge carries a null"),
        Value::Array(a) => a.iter().for_each(assert_no_null),
        Value::Object(o) => o.values().for_each(assert_no_null),
        _ => {}
    }
}

// ------------------------------------------------------------ steps

async fn register_options(app: &Router, code: &str, name: &str) -> Reply {
    call(
        app,
        None,
        "POST",
        "/api/auth/register/options",
        Some(json!({ "code": code, "displayName": name })),
    )
    .await
}

/// The answer to a registration challenge, ready to post.
fn answer_registration(options: &Reply, key: &mut Authenticator) -> Value {
    assert_eq!(options.status, StatusCode::OK, "{}", options.body);
    let credential = key.create(&options.body["data"]["options"]);
    json!({ "ceremonyId": options.body["data"]["ceremonyId"], "credential": credential })
}

/// Create an account with `code`; returns the reply of the last step.
async fn register(app: &Router, code: &str, name: &str, key: &mut Authenticator) -> Reply {
    let options = register_options(app, code, name).await;
    let answer = answer_registration(&options, key);
    call(app, None, "POST", "/api/auth/register", Some(answer)).await
}

/// An answer to a fresh sign-in challenge, ready to post.
async fn sign_in_answer(app: &Router, key: &mut Authenticator) -> Value {
    let r = call(app, None, "POST", "/api/auth/sign-in/options", None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let credential = key.get(&r.body["data"]["options"]);
    json!({ "ceremonyId": r.body["data"]["ceremonyId"], "credential": credential })
}

async fn me(app: &Router, token: &str) -> Reply {
    call(app, Some(token), "GET", "/api/me", None).await
}

async fn gm_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM gms")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A fresh instance with setup open, and its first GM created.
async fn first_gm(pool: &PgPool) -> (Router, Authenticator, String) {
    let app = common::app_with(pool.clone(), SetupState::open_with(SETUP_CODE.into()));
    let mut key = Authenticator::new();
    let r = register(&app, SETUP_CODE, "Romain", &mut key).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let token = r.session_token().expect("session cookie");
    (app, key, token)
}

// ------------------------------------------------------------ tests

#[tokio::test]
async fn the_first_gm_needs_the_setup_code_then_setup_closes() {
    let pool = common::fresh_instance_pool().await;
    let app = common::app_with(pool.clone(), SetupState::open_with(SETUP_CODE.into()));

    let r = call(&app, None, "GET", "/api/auth/status", None).await;
    assert_eq!(r.body["data"]["needsSetup"], true);

    // Reaching the server first is not enough.
    let r = register_options(&app, "a-guess", "Mallory").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.body["error"]["code"], "INVALID_REGISTRATION_CODE");

    let mut key = Authenticator::new();
    let r = register(&app, SETUP_CODE, "  Romain  ", &mut key).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["data"]["displayName"], "Romain");
    let cookie = r.set_cookie.clone().unwrap();
    for flag in ["HttpOnly", "SameSite=Strict", "Path=/api"] {
        assert!(cookie.contains(flag), "{flag} missing from {cookie}");
    }
    assert!(
        !cookie.contains("Secure"),
        "plain-HTTP dev origin: {cookie}"
    );
    let token = r.session_token().unwrap();
    let r = me(&app, &token).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["data"]["displayName"], "Romain");

    let r = call(&app, None, "GET", "/api/auth/status", None).await;
    assert_eq!(r.body["data"]["needsSetup"], false);

    // The setup code is spent: it creates no second account.
    let r = register_options(&app, SETUP_CODE, "Mallory").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(gm_count(&pool).await, 1);
}

#[tokio::test]
async fn two_racing_setups_create_one_account() {
    let pool = common::fresh_instance_pool().await;
    let app = common::app_with(pool.clone(), SetupState::open_with(SETUP_CODE.into()));
    let (mut a, mut b) = (Authenticator::new(), Authenticator::new());

    // Both get a challenge while no account exists yet…
    let first = register_options(&app, SETUP_CODE, "A").await;
    let second = register_options(&app, SETUP_CODE, "B").await;
    let first = answer_registration(&first, &mut a);
    let second = answer_registration(&second, &mut b);

    // …but only the first answer creates one.
    let r = call(&app, None, "POST", "/api/auth/register", Some(first)).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let r = call(&app, None, "POST", "/api/auth/register", Some(second)).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "INVALID_REGISTRATION_CODE");
    assert!(r.set_cookie.is_none());
    assert_eq!(gm_count(&pool).await, 1);
}

#[tokio::test]
async fn a_registration_answer_works_once() {
    let pool = common::fresh_instance_pool().await;
    let app = common::app_with(pool.clone(), SetupState::open_with(SETUP_CODE.into()));
    let mut key = Authenticator::new();
    let options = register_options(&app, SETUP_CODE, "Romain").await;
    let answer = answer_registration(&options, &mut key);

    let r = call(
        &app,
        None,
        "POST",
        "/api/auth/register",
        Some(answer.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = call(&app, None, "POST", "/api/auth/register", Some(answer)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "INVALID_PASSKEY");
    assert_eq!(gm_count(&pool).await, 1);
}

#[tokio::test]
async fn a_passkey_signs_in_and_each_answer_works_once() {
    let pool = common::fresh_instance_pool().await;
    let (app, mut key, _) = first_gm(&pool).await;

    let answer = sign_in_answer(&app, &mut key).await;
    let r = call(
        &app,
        None,
        "POST",
        "/api/auth/sign-in",
        Some(answer.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["data"]["displayName"], "Romain");
    let token = r.session_token().expect("session cookie");
    assert_eq!(me(&app, &token).await.body["data"]["displayName"], "Romain");

    // The session is stored as a hash, never as the token itself.
    let stored: Vec<String> = sqlx::query_scalar("SELECT token_hash FROM gm_sessions")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(stored.contains(&hash_token(&token)));
    assert!(!stored.contains(&token));

    // The same answer again: the ceremony is spent.
    let r = call(&app, None, "POST", "/api/auth/sign-in", Some(answer)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.body["error"]["code"], "INVALID_PASSKEY");
    assert!(r.set_cookie.is_none());

    // A later sign-in with the same key still works (counter moved on).
    let answer = sign_in_answer(&app, &mut key).await;
    let r = call(&app, None, "POST", "/api/auth/sign-in", Some(answer)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
}

#[tokio::test]
async fn a_passkey_cannot_sign_in_as_another_gm() {
    let pool = common::fresh_instance_pool().await;
    let (app, mut key, _) = first_gm(&pool).await;
    let (other, _) = common::signed_in_gm(&pool, "Other").await;

    // The authenticator claims to be the other GM's: the credential is
    // not theirs, so nothing verifies against it.
    let mut answer = sign_in_answer(&app, &mut key).await;
    answer["credential"]["response"]["userHandle"] = json!(base64_url(other.as_bytes()));
    let r = call(&app, None, "POST", "/api/auth/sign-in", Some(answer)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{}", r.body);
    assert!(r.set_cookie.is_none());
}

#[tokio::test]
async fn an_unregistered_passkey_does_not_sign_in() {
    let pool = common::fresh_instance_pool().await;
    let (app, _, _) = first_gm(&pool).await;

    // A key registered on another instance, claiming an account here.
    let elsewhere = common::fresh_instance_pool().await;
    let (_, mut stranger, _) = first_gm(&elsewhere).await;
    let answer = sign_in_answer(&app, &mut stranger).await;
    let r = call(&app, None, "POST", "/api/auth/sign-in", Some(answer)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{}", r.body);
}

#[tokio::test]
async fn an_invitation_admits_one_more_gm_once() {
    let pool = common::fresh_instance_pool().await;
    let (app, _, romain) = first_gm(&pool).await;

    let r = call(&app, Some(&romain), "POST", "/api/gm-invites", None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let code = r.body["data"]["code"].as_str().unwrap().to_string();

    let mut key = Authenticator::new();
    let r = register(&app, &code, "Marc", &mut key).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let marc = r.session_token().unwrap();
    assert_eq!(me(&app, &marc).await.body["data"]["displayName"], "Marc");

    // Marc signs in with his own passkey, as himself.
    let answer = sign_in_answer(&app, &mut key).await;
    let r = call(&app, None, "POST", "/api/auth/sign-in", Some(answer)).await;
    assert_eq!(r.body["data"]["displayName"], "Marc");

    // Used: the code admits nobody else, and leaves Romain's list.
    let r = register_options(&app, &code, "Mallory").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(&app, Some(&romain), "GET", "/api/gm-invites", None).await;
    assert_eq!(r.body["data"], json!([]));
    assert_eq!(gm_count(&pool).await, 2);
}

#[tokio::test]
async fn an_invitation_revoked_during_the_ceremony_admits_nobody() {
    let pool = common::fresh_instance_pool().await;
    let (app, _, romain) = first_gm(&pool).await;
    let r = call(&app, Some(&romain), "POST", "/api/gm-invites", None).await;
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    let id = r.body["data"]["id"].as_str().unwrap().to_string();

    let mut key = Authenticator::new();
    let options = register_options(&app, &code, "Marc").await;
    let answer = answer_registration(&options, &mut key);
    let r = call(
        &app,
        Some(&romain),
        "DELETE",
        &format!("/api/gm-invites/{id}"),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);

    let r = call(&app, None, "POST", "/api/auth/register", Some(answer)).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);
    assert_eq!(r.body["error"]["code"], "INVALID_REGISTRATION_CODE");
    assert_eq!(gm_count(&pool).await, 1);
}

#[tokio::test]
async fn an_expired_invitation_admits_nobody() {
    let pool = common::fresh_instance_pool().await;
    let (app, _, romain) = first_gm(&pool).await;
    let r = call(&app, Some(&romain), "POST", "/api/gm-invites", None).await;
    let code = r.body["data"]["code"].as_str().unwrap().to_string();
    sqlx::query("UPDATE gm_invites SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();

    let r = register_options(&app, &code, "Marc").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_display_name_is_required() {
    let pool = common::fresh_instance_pool().await;
    let app = common::app_with(pool, SetupState::open_with(SETUP_CODE.into()));
    let r = register_options(&app, SETUP_CODE, "   ").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"]["code"], "INVALID_DISPLAY_NAME");
    let r = register_options(&app, SETUP_CODE, &"x".repeat(61)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn over_https_the_session_cookie_is_secure() {
    const HTTPS: &str = "https://promptus.example.test";
    let pool = common::fresh_instance_pool().await;
    let auth =
        promptus_back::state::Auth::new(HTTPS, None, SetupState::open_with(SETUP_CODE.into()))
            .unwrap();
    let app = promptus_back::app::router(
        promptus_back::state::AppState {
            pool,
            auth,
            live: promptus_back::live::LiveHub::new(Default::default()),
        },
        &[],
    );
    let mut key = Authenticator::at(HTTPS);

    let r = register(&app, SETUP_CODE, "Romain", &mut key).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert!(r.set_cookie.unwrap().contains("; Secure"));
}

fn base64_url(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
