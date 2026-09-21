//! Getting a message to the relay, over the channel the credential asks for.
//!
//! The SMTP conversation itself belongs to mailbourne. This is only the part
//! mailbourne leaves to its caller on purpose: opening the socket and
//! deciding what, if anything, wraps it. That decision cannot live in a mail
//! engine, because it is a question about *this deployment* — whether the
//! relay is a public host with a signed certificate, a box on a private
//! network with a self-signed one, or a sink on localhost.
//!
//! Four answers, and the credential names one:
//!
//! | `tls` | what happens | when it is right |
//! |---|---|---|
//! | `starttls` | connect plain, upgrade, verify | port 587, a real relay. The default |
//! | `tls` | encrypted from the first byte, verify | port 465 |
//! | `starttls-insecure` | upgrade, but check nothing | your own server before it has a signed certificate |
//! | `none` | no encryption at all | a test sink on localhost, never a relay |
//!
//! The two `starttls` modes go through mailbourne's
//! [`submit_with_starttls`], which refuses to continue when the relay does
//! not offer the upgrade — the password would be next. The other two have
//! already settled the channel, so they use [`submit`].

use std::sync::Arc;

use mailbourne::send::conversation::{
    Credentials, Outcome, deliver, deliver_with_starttls, submit, submit_with_starttls,
};
use mailbourne::shared::core::{Envelope, Message};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};

use crate::pipeline::PipelineError;

/// What the credential says about encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Connect in the clear, then `STARTTLS`, verifying the certificate.
    StartTls,
    /// TLS from the first byte, verifying the certificate.
    ImplicitTls,
    /// `STARTTLS`, but accept any certificate and any hostname.
    StartTlsInsecure,
    /// No encryption.
    None,
}

impl Channel {
    /// Reads the credential's `tls` field. Empty means the safe default.
    pub fn parse(raw: &str) -> Result<Self, PipelineError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "starttls" => Ok(Self::StartTls),
            "tls" => Ok(Self::ImplicitTls),
            "starttls-insecure" => Ok(Self::StartTlsInsecure),
            "none" => Ok(Self::None),
            other => Err(PipelineError::new(
                "FW_NODE_MAIL_CREDENTIAL",
                format!("unknown tls mode '{other}': use starttls, tls, starttls-insecure, or none"),
            )),
        }
    }

    /// Whether this channel protects the password from someone on the wire.
    pub fn is_private(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Accepts any certificate, for [`Channel::StartTlsInsecure`].
///
/// This exists for one real case: a mail server you run yourself, before it
/// has a certificate a public authority signed — a fresh mailbourne, whose
/// STARTTLS is self-signed on first boot. Such a server rightly refuses a
/// password over a plain connection, so `none` cannot reach it, and without
/// this the choice would be between no encryption and no test.
///
/// What it costs: the traffic is protected from someone listening, and
/// defenceless against someone interposing, who can present any certificate
/// and read the password. Use it on a private network — a localhost, a
/// tunnel, a VPN — and move to `starttls` when the server has a real
/// certificate.
#[derive(Debug)]
struct AcceptAnyCertificate(Arc<tokio_rustls::rustls::crypto::CryptoProvider>);

impl tokio_rustls::rustls::client::danger::ServerCertVerifier for AcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: tokio_rustls::rustls::pki_types::UnixTime,
    ) -> Result<tokio_rustls::rustls::client::danger::ServerCertVerified, tokio_rustls::rustls::Error>
    {
        Ok(tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<tokio_rustls::rustls::client::danger::HandshakeSignatureValid, tokio_rustls::rustls::Error>
    {
        tokio_rustls::rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        dss: &tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<tokio_rustls::rustls::client::danger::HandshakeSignatureValid, tokio_rustls::rustls::Error>
    {
        tokio_rustls::rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn provider() -> Arc<tokio_rustls::rustls::crypto::CryptoProvider> {
    Arc::new(tokio_rustls::rustls::crypto::ring::default_provider())
}

/// The TLS client for a channel that verifies certificates.
fn verifying_connector() -> Result<TlsConnector, PipelineError> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| PipelineError::new("FW_NODE_MAIL_TRANSPORT", e.to_string()))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

/// The TLS client that checks nothing. See [`AcceptAnyCertificate`].
fn trusting_connector() -> Result<TlsConnector, PipelineError> {
    let provider = provider();
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| PipelineError::new("FW_NODE_MAIL_TRANSPORT", e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate(provider)))
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

/// A name for the TLS handshake. An IP or a tunnel's `localhost` is not a
/// name a certificate can carry, so the insecure channel substitutes a
/// placeholder — it verifies nothing anyway.
fn server_name(host: &str) -> Result<ServerName<'static>, PipelineError> {
    ServerName::try_from(host.to_string()).map_err(|e| {
        PipelineError::new("FW_NODE_MAIL_TRANSPORT", format!("'{host}' is not a valid server name: {e}"))
    })
}

/// Opens the channel and hands the message over.
///
/// With no credentials it never authenticates — a relay on localhost that
/// carries mail for anyone who can reach it, which is what a test sink is.
///
/// Answers mailbourne's [`Outcome`], so the caller can tell a relay saying
/// "not now" from one saying "never", and a rejected password from a
/// rejected letter.
pub async fn deliver_to_relay(
    host: &str,
    port: u16,
    channel: Channel,
    our_hostname: &str,
    credentials: Option<&Credentials>,
    envelope: &Envelope,
    message: &Message,
) -> Result<Outcome, PipelineError> {
    let stream = TcpStream::connect((host, port)).await.map_err(|e| {
        PipelineError::new("FW_NODE_MAIL_TRANSPORT", format!("cannot reach {host}:{port}: {e}"))
    })?;
    // Two different failures reach this function: the TLS handshake gives an
    // io error, the SMTP dialogue gives a ReplyError. Both mean the wire
    // broke rather than the relay refusing, so both carry the same code.
    let io = |e: std::io::Error| PipelineError::new("FW_NODE_MAIL_SEND", format!("smtp: {e}"));
    let wire = |e: mailbourne::send::conversation::ReplyError| {
        PipelineError::new("FW_NODE_MAIL_SEND", format!("smtp: {e:?}"))
    };

    match channel {
        Channel::StartTls => {
            let connector = verifying_connector()?;
            let name = server_name(host)?;
            match credentials {
                Some(credentials) => submit_with_starttls(
                    stream,
                    move |s| async move { connector.connect(name, s).await },
                    our_hostname,
                    credentials,
                    envelope,
                    message,
                )
                .await
                .map_err(wire),
                None => deliver_with_starttls(
                    stream,
                    move |s| async move { connector.connect(name, s).await },
                    our_hostname,
                    envelope,
                    message,
                )
                .await
                .map_err(wire),
            }
        }
        Channel::StartTlsInsecure => {
            let connector = trusting_connector()?;
            // The name is never checked on this path, so a host that is an
            // IP or a tunnel endpoint still completes the handshake.
            let name = server_name(host).or_else(|_| server_name("localhost"))?;
            match credentials {
                Some(credentials) => submit_with_starttls(
                    stream,
                    move |s| async move { connector.connect(name, s).await },
                    our_hostname,
                    credentials,
                    envelope,
                    message,
                )
                .await
                .map_err(wire),
                None => deliver_with_starttls(
                    stream,
                    move |s| async move { connector.connect(name, s).await },
                    our_hostname,
                    envelope,
                    message,
                )
                .await
                .map_err(wire),
            }
        }
        Channel::ImplicitTls => {
            let connector = verifying_connector()?;
            let secured = connector.connect(server_name(host)?, stream).await.map_err(io)?;
            match credentials {
                Some(credentials) => submit(secured, our_hostname, credentials, envelope, message)
                    .await
                    .map_err(wire),
                None => deliver(secured, our_hostname, envelope, message).await.map_err(wire),
            }
        }
        Channel::None => match credentials {
            Some(credentials) => submit(stream, our_hostname, credentials, envelope, message)
                .await
                .map_err(wire),
            None => deliver(stream, our_hostname, envelope, message).await.map_err(wire),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_credential_names_the_channel_and_an_unknown_one_is_refused() {
        assert_eq!(Channel::parse("").unwrap(), Channel::StartTls);
        assert_eq!(Channel::parse("  STARTTLS ").unwrap(), Channel::StartTls);
        assert_eq!(Channel::parse("tls").unwrap(), Channel::ImplicitTls);
        assert_eq!(Channel::parse("starttls-insecure").unwrap(), Channel::StartTlsInsecure);
        assert_eq!(Channel::parse("none").unwrap(), Channel::None);
        let err = Channel::parse("ssl").unwrap_err();
        assert_eq!(err.code, "FW_NODE_MAIL_CREDENTIAL");
        assert!(err.message.contains("starttls, tls, starttls-insecure, or none"), "{}", err.message);
    }

    #[test]
    fn only_the_none_channel_leaves_the_password_readable() {
        assert!(Channel::StartTls.is_private());
        assert!(Channel::ImplicitTls.is_private());
        assert!(Channel::StartTlsInsecure.is_private());
        assert!(!Channel::None.is_private());
    }

    #[test]
    fn both_tls_clients_build() {
        assert!(verifying_connector().is_ok());
        assert!(trusting_connector().is_ok());
    }

    #[test]
    fn a_hostname_is_accepted_and_something_that_cannot_be_one_is_named() {
        assert!(server_name("mail.example.com").is_ok());
        assert!(server_name("127.0.0.1").is_ok());
        let err = server_name("not a host name").unwrap_err();
        assert!(err.message.contains("not a valid server name"), "{}", err.message);
    }
}
