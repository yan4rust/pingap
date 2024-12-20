// Copyright 2024 Tree xie.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::util;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
pub static LOG_CATEGORY: &str = "acme";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Instant error, category: {category}, {source}"))]
    Instant {
        category: String,
        source: instant_acme::Error,
    },
    #[snafu(display("Rcgen error, category: {category}, {source}"))]
    Rcgen {
        category: String,
        source: rcgen::Error,
    },
    #[snafu(display("Challenge not found error, {message}"))]
    NotFound { message: String },
    #[snafu(display("Lets encrypt fail, category: {category}, {message}"))]
    Fail { category: String, message: String },
    #[snafu(display("X509 error, category: {category}, {message}"))]
    X509 { category: String, message: String },
}

type Result<T, E = Error> = std::result::Result<T, E>;

fn parse_ip_addr(data: &[u8]) -> Result<IpAddr> {
    let addr = if data.len() == 4 {
        let arr: [u8; 4] = data.try_into().unwrap_or_default();
        IpAddr::V4(Ipv4Addr::from(arr))
    } else {
        let arr: [u8; 16] = data.try_into().unwrap_or_default();
        IpAddr::V6(Ipv6Addr::from(arr))
    };
    Ok(addr)
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct Certificate {
    pub domains: Vec<String>,
    pub pem: Vec<u8>,
    pub key: Vec<u8>,
    pub acme: Option<String>,
    pub not_after: i64,
    pub not_before: i64,
    pub issuer: String,
}
impl Certificate {
    pub fn new(pem: String, key: String) -> Result<Certificate> {
        let pem_data =
            util::convert_certificate_bytes(&Some(pem)).unwrap_or_default();
        let (_, p) =
            x509_parser::pem::parse_x509_pem(&pem_data).map_err(|e| {
                Error::X509 {
                    category: "parse_x509_pem".to_string(),
                    message: e.to_string(),
                }
            })?;
        let x509 = p.parse_x509().map_err(|e| Error::X509 {
            category: "parse_x509".to_string(),
            message: e.to_string(),
        })?;
        let mut dns_names = vec![];
        if let Ok(Some(subject_alternative_name)) =
            x509.subject_alternative_name()
        {
            for item in subject_alternative_name.value.general_names.iter() {
                match item {
                    x509_parser::prelude::GeneralName::DNSName(name) => {
                        dns_names.push(name.to_string());
                    },
                    x509_parser::prelude::GeneralName::IPAddress(data) => {
                        if let Ok(addr) = parse_ip_addr(data) {
                            dns_names.push(addr.to_string());
                        }
                    },
                    _ => {},
                };
            }
        };
        let validity = x509.validity();
        Ok(Self {
            domains: dns_names,
            pem: pem_data,
            key: util::convert_certificate_bytes(&Some(key))
                .unwrap_or_default(),
            not_after: validity.not_after.timestamp(),
            not_before: validity.not_before.timestamp(),
            issuer: x509.issuer.to_string(),
            ..Default::default()
        })
    }
    /// Get the common name of certificate issuer
    pub fn get_issuer_common_name(&self) -> String {
        let re = regex::Regex::new(r"CN=(?P<CN>[\S ]+?)($|,)").unwrap();
        if let Some(caps) = re.captures(&self.issuer) {
            return caps["CN"].to_string();
        }
        "".to_string()
    }
    /// Validate the cert is within the expiration date.
    pub fn valid(&self) -> bool {
        let ts = util::now().as_secs() as i64;
        self.not_after - ts > 2 * 24 * 3600
    }
    /// Get the cert pem data.
    pub fn get_cert(&self) -> Vec<u8> {
        self.pem.clone()
    }
    /// Get the cert key data.
    pub fn get_key(&self) -> Vec<u8> {
        self.key.clone()
    }
}

pub fn get_token_path(key: &str) -> String {
    format!("pingap-acme-tokens/${key}")
}

mod lets_encrypt;
mod validity_checker;

pub use lets_encrypt::{handle_lets_encrypt, new_lets_encrypt_service};
pub use validity_checker::new_tls_validity_service;

#[cfg(test)]
mod tests {
    use super::Certificate;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_cert() {
        // spellchecker:off
        let pem = r###"-----BEGIN CERTIFICATE-----
MIID/TCCAmWgAwIBAgIQJUGCkB1VAYha6fGExkx0KTANBgkqhkiG9w0BAQsFADBV
MR4wHAYDVQQKExVta2NlcnQgZGV2ZWxvcG1lbnQgQ0ExFTATBgNVBAsMDHZpY2Fu
c29AdHJlZTEcMBoGA1UEAwwTbWtjZXJ0IHZpY2Fuc29AdHJlZTAeFw0yNDA3MDYw
MjIzMzZaFw0yNjEwMDYwMjIzMzZaMEAxJzAlBgNVBAoTHm1rY2VydCBkZXZlbG9w
bWVudCBjZXJ0aWZpY2F0ZTEVMBMGA1UECwwMdmljYW5zb0B0cmVlMIIBIjANBgkq
hkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAv5dbylSPQNARrpT/Rn7qZf6JmH3cueMp
YdOpctuPYeefT0Jdgp67bg17fU5pfyR2BWYdwyvHCNmKqLdYPx/J69hwTiVFMOcw
lVQJjbzSy8r5r2cSBMMsRaAZopRDnPy7Ls7Ji+AIT4vshUgL55eR7ACuIJpdtUYm
TzMx9PTA0BUDkit6z7bTMaEbjDmciIBDfepV4goHmvyBJoYMIjnAwnTFRGRs/QJN
d2ikFq999fRINzTDbRDP1K0Kk6+zYoFAiCMs9lEDymu3RmiWXBXpINR/Sv8CXtz2
9RTVwTkjyiMOPY99qBfaZTiy+VCjcwTGKPyus1axRMff4xjgOBewOwIDAQABo14w
XDAOBgNVHQ8BAf8EBAMCBaAwEwYDVR0lBAwwCgYIKwYBBQUHAwEwHwYDVR0jBBgw
FoAUhU5Igu3uLUabIqUhUpVXjk1JVtkwFAYDVR0RBA0wC4IJcGluZ2FwLmlvMA0G
CSqGSIb3DQEBCwUAA4IBgQDBimRKrqnEG65imKriM2QRCEfdB6F/eP9HYvPswuAP
tvQ6m19/74qbtkd6vjnf6RhMbj9XbCcAJIhRdnXmS0vsBrLDsm2q98zpg6D04F2E
L++xTiKU6F5KtejXcTHHe23ZpmD2XilwcVDeGFu5BEiFoRH9dmqefGZn3NIwnIeD
Yi31/cL7BoBjdWku5Qm2nCSWqy12ywbZtQCbgbzb8Me5XZajeGWKb8r6D0Nb+9I9
OG7dha1L3kxerI5VzVKSiAdGU0C+WcuxfsKAP8ajb1TLOlBaVyilfqmiF457yo/2
PmTYzMc80+cQWf7loJPskyWvQyfmAnSUX0DI56avXH8LlQ57QebllOtKgMiCo7cr
CCB2C+8hgRNG9ZmW1KU8rxkzoddHmSB8d6+vFqOajxGdyOV+aX00k3w6FgtHOoKD
Ztdj1N0eTfn02pibVcXXfwESPUzcjERaMAGg1hoH1F4Gxg0mqmbySAuVRqNLnXp5
CRVQZGgOQL6WDg3tUUDXYOs=
-----END CERTIFICATE-----"###;
        // spellchecker:on
        let cert = Certificate::new(pem.to_string(), "".to_string()).unwrap();

        assert_eq!(
            "O=mkcert development CA, OU=vicanso@tree, CN=mkcert vicanso@tree",
            cert.issuer
        );
        assert_eq!(1720232616, cert.not_before);
        assert_eq!(1791253416, cert.not_after);
        assert_eq!("mkcert vicanso@tree", cert.get_issuer_common_name());
    }
}
