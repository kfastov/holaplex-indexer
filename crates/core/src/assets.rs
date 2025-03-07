//! ``AssetIdentifier`` utils - Parse and capture tx and cid

use std::borrow::Cow;

use cid::Cid;
use url::Url;

/// An Arweave transaction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArTxid(pub [u8; 32]);

/// Struct to hold tx ids
#[derive(Debug, Clone)]
pub struct AssetIdentifier<'a> {
    /// The CID of a potential IPFS asset
    pub ipfs: Option<(Cid, String)>,
    /// The transaction ID of a potential Arweave asset
    pub arweave: Option<(ArTxid, String)>,
    /// The URL that was parsed
    pub url: &'a Url,
}

/// An unambiguous asset-type hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetHint {
    /// The asset is expected to be an IPFS CID
    Ipfs,
    /// The asset is expected to be an Arweave transaction
    Arweave,
}

impl<'a> AssetIdentifier<'a> {
    /// Attempt to parse IPFS or Arweave asset IDs from a URL.
    ///
    /// Parsing occurs as follows:
    ///  - If the URL contains a CID in any segment, it is considered to be an
    ///    IPFS URL.
    ///  - If the URL contains a base64-encoded 256-bit digest, it is
    ///    considered to be an Arweave transaction.
    ///  - If both of the above are found, the URL is considered ambiguous but
    ///    usable and both Arweave and IPFS parse results are stored.
    ///  - If more than one valid IPFS parse result is found, the IPFS result is
    ///    considered ambiguous and unusable and no IPFS data is returned.  The
    ///    same holds for the Arweave parse result.
    #[must_use]
    pub fn new(url: &'a Url) -> Self {
        let mut ipfs = Ok(None);
        let mut arweave = Ok(None);

        Self::visit_url(url, |s, i| {
            let slice_path = || {
                i.and_then(|i| url.path_segments().map(|s| (i, s)))
                    .map_or_else(String::new, |(i, s)| s.skip(i).intersperse("/").collect())
            };

            if let Some(c) = Self::try_ipfs(s) {
                Self::advance_heuristic(&mut ipfs, (c, slice_path()));
            }

            if let Some(t) = Self::try_arweave(s) {
                Self::advance_heuristic(&mut arweave, (t, slice_path()));
            }
        });

        Self {
            ipfs: ipfs.ok().flatten(),
            arweave: arweave.ok().flatten(),
            url,
        }
    }

    fn visit_url(url: &Url, mut f: impl FnMut(&str, Option<usize>)) {
        Some(url.scheme())
            .into_iter()
            .chain(url.domain().into_iter().flat_map(|s| s.split('.')))
            .chain(Some(url.username()))
            .chain(url.password())
            .map(|s| (s, Some(0)))
            .chain(Some((url.path(), None)))
            .chain(
                url.path_segments()
                    .into_iter()
                    .flat_map(|s| s.into_iter().enumerate().map(|(i, s)| (s, Some(i + 1)))),
            )
            .chain(url.query().map(|q| (q, Some(0))))
            .chain(url.fragment().map(|f| (f, Some(0))))
            .for_each(|(s, i)| f(s, i));

        url.query_pairs().for_each(|(k, v)| {
            f(k.as_ref(), Some(0));
            f(v.as_ref(), Some(0));
        });
    }

    fn try_ipfs(s: &str) -> Option<Cid> {
        s.try_into().ok()
    }

    fn try_arweave(s: &str) -> Option<ArTxid> {
        [
            base64::URL_SAFE,
            base64::URL_SAFE_NO_PAD,
            base64::STANDARD,
            base64::STANDARD_NO_PAD,
        ]
        .into_iter()
        .find_map(|c| {
            base64::decode_config(s.as_bytes(), c)
                .ok()
                .and_then(|v| v.try_into().ok())
                .map(ArTxid)
        })
    }

    fn advance_heuristic<T: Eq>(state: &mut Result<Option<T>, ()>, value: T) {
        match state {
            // We found a match
            Ok(None) => *state = Ok(Some(value)),
            // We found two identical matches, no change is necessary
            Ok(Some(v)) if *v == value => (),
            // We found two differing matches, convert to error due to ambiguity
            Ok(Some(_)) => *state = Err(()),
            Err(()) => (),
        }
    }

    /// Generate a binary fingerprint for this asset ID.
    ///
    /// For ambiguous cases, a type hint must be provided for disambiguation
    /// otherwise no result is returned.
    #[must_use]
    pub fn fingerprint(&self, hint: Option<AssetHint>, allow_indet: bool) -> Option<Cow<[u8]>> {
        match (self.ipfs.as_ref(), self.arweave.as_ref(), hint) {
            (Some((cid, path)), Some(_), Some(AssetHint::Ipfs)) | (Some((cid, path)), None, _) => {
                Some(Cow::Owned(Self::fingerprint_ipfs(cid, path)))
            },
            (Some(_), Some((txid, path)), Some(AssetHint::Arweave))
            | (None, Some((txid, path)), _) => Some(Self::fingerprint_arweave(txid, path)),
            (None, None, _) if allow_indet => {
                Some(Cow::Owned(Self::fingerprint_indeterminate(self.url)))
            },
            (Some(_), Some(_), None) | (None, None, _) => None,
        }
    }

    /// Return each possible fingerprint for this asset ID, alongside its
    /// corresponding [`AssetHint`] (or `None` if the fingerprint represents
    /// an indeterminate asset).
    pub fn fingerprints_hinted(&self) -> impl Iterator<Item = (Cow<[u8]>, Option<AssetHint>)> {
        self.ipfs
            .iter()
            .map(|(c, p)| {
                (
                    Cow::Owned(Self::fingerprint_ipfs(c, p)),
                    Some(AssetHint::Ipfs),
                )
            })
            .chain(
                self.arweave
                    .iter()
                    .map(|(t, p)| (Self::fingerprint_arweave(t, p), Some(AssetHint::Arweave))),
            )
            .chain(std::iter::once_with(|| {
                (Cow::Owned(Self::fingerprint_indeterminate(self.url)), None)
            }))
    }

    fn fingerprint_ipfs(cid: &Cid, path: &str) -> Vec<u8> {
        if path.is_empty() {
            use cid::multihash::Hasher;

            let mut h = cid::multihash::Sha2_256::default();

            cid.write_bytes(&mut h).unwrap_or_else(|_| unreachable!());
            h.update(path.as_bytes());

            h.finalize().as_ref().to_vec()
        } else {
            cid.to_bytes()
        }
    }

    fn fingerprint_arweave<'b>(txid: &'b ArTxid, path: &'_ str) -> Cow<'b, [u8]> {
        if path.is_empty() {
            Cow::Borrowed(&txid.0)
        } else {
            use cid::multihash::Hasher;

            let mut h = cid::multihash::Sha2_256::default();

            h.update(&txid.0);
            h.update(path.as_bytes());

            Cow::Owned(h.finalize().as_ref().to_vec())
        }
    }

    fn fingerprint_indeterminate(url: &Url) -> Vec<u8> {
        use cid::multihash::Hasher;

        let mut h = cid::multihash::Sha3_256::default();

        h.update(url.as_str().as_bytes());

        h.finalize().as_ref().to_vec()
    }
}

#[cfg(feature = "asset-cdn")]
mod cdn {
    use super::{AssetHint, AssetIdentifier, Url};
    use crate::prelude::*;

    /// Supported width sizes for asset proxy
    #[derive(Debug, Clone, Copy, strum::FromRepr)]
    #[repr(i32)]
    pub enum ImageSize {
        /// image natural size
        Original = 0,
        /// tiny image
        Tiny = 100,
        /// extra small image
        XSmall = 400,
        /// small image
        Small = 600,
        /// medium image
        Medium = 800,
        /// large image
        Large = 1400,
    }

    impl From<i32> for ImageSize {
        fn from(value: i32) -> Self {
            Self::from_repr(value).unwrap_or(Self::Original)
        }
    }

    /// Common arguments for binaries using [`proxy_url`]
    #[derive(Debug, Clone, clap::Args)]
    pub struct AssetProxyArgs {
        /// Endpoint for Holaplex asset CDN
        #[arg(long, env)]
        asset_proxy_endpoint: String,

        /// Number of replicas available to proxy asset requests to
        #[arg(long, env)]
        asset_proxy_count: u8,
    }

    fn format_impl<'p, 'q>(
        args: &AssetProxyArgs,
        id: &AssetIdentifier,
        hint: AssetHint,
        path: impl IntoIterator<Item = &'p str>,
        query: impl IntoIterator<Item = (&'q str, &'q str)>,
    ) -> Result<Url> {
        let rem = md5::compute(
            id.fingerprint(Some(hint), false)
                .unwrap_or_else(|| unreachable!())
                .as_ref(),
        )[0]
        .rem_euclid(args.asset_proxy_count);
        let assets_cdn = &args.asset_proxy_endpoint;

        let mut url = Url::parse(&assets_cdn.replace(
            "[n]",
            &if rem == 0 {
                String::new()
            } else {
                rem.to_string()
            },
        ))
        .context("Invalid asset proxy URL")?;

        url.path_segments_mut()
            .unwrap_or_else(|_| unreachable!())
            .extend(path);
        url.query_pairs_mut().extend_pairs(query);

        Ok(url)
    }

    /// Format an [`AssetIdentifier`] as an Holaplex asset proxy URL.  Returns
    /// `None` if the ID was unparseable or ambiguous.
    ///
    /// # Errors
    /// This function fails if the asset proxy configured by `args` has an
    /// invalid URL
    pub fn proxy_url_hinted<'a>(
        args: &AssetProxyArgs,
        id: &'a AssetIdentifier,
        hint: impl Into<Option<AssetHint>>,
        query: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Option<Url>> {
        match (&id.arweave, &id.ipfs, hint.into()) {
            (Some(_), Some(_), None) => {
                warn!("Ambiguous asset ID {:?} encountered", id);
                Ok(None)
            },
            (None, None, _) => Ok(None),
            (Some((txid, path)), None, _)
            | (Some((txid, path)), Some(_), Some(AssetHint::Arweave)) => {
                let txid = base64::encode_config(txid.0, base64::URL_SAFE_NO_PAD);

                format_impl(
                    args,
                    id,
                    AssetHint::Arweave,
                    ["arweave", &txid],
                    query.into_iter().chain(if path.is_empty() {
                        None
                    } else {
                        Some(("path", &**path))
                    }),
                )
                .map(Some)
            },
            (None, Some((cid, path)), _) | (Some(_), Some((cid, path)), Some(AssetHint::Ipfs)) => {
                let cid = cid.to_string();

                format_impl(
                    args,
                    id,
                    AssetHint::Ipfs,
                    ["ipfs", &cid],
                    query.into_iter().chain(if path.is_empty() {
                        None
                    } else {
                        Some(("path", &**path))
                    }),
                )
                .map(Some)
            },
        }
    }

    /// Get the base URL for proxied Twitter handle requests
    ///
    /// # Errors
    /// This function fails if the asset proxy configured by `args` has an
    /// invalid URL
    #[inline]
    pub fn proxy_twitter_handle_url(
        args: &AssetProxyArgs,
        screen_name: impl AsRef<str>,
    ) -> Result<Url> {
        let mut url = Url::parse(&args.asset_proxy_endpoint.replace("[n]", ""))
            .context("Invalid asset proxy URL")?;

        url.path_segments_mut()
            .unwrap_or_else(|_| unreachable!())
            .extend(["twitter", screen_name.as_ref()]);

        Ok(url)
    }
    /// Get the proxy URL parameters for non-permaweb assets
    ///
    /// # Errors
    /// This function fails if the asset proxy configured by `args` has an
    /// invalid URL

    #[inline]
    pub fn proxy_non_permaweb_url(args: &AssetProxyArgs, endpoint: impl AsRef<str>) -> Result<Url> {
        let mut url = Url::parse(&args.asset_proxy_endpoint.replace("[n]", ""))
            .context("Invalid asset proxy URL")?;
        url.query_pairs_mut().append_pair("url", endpoint.as_ref());
        Ok(url)
    }

    /// Format an [`AssetIdentifier`] as an Holaplex asset proxy URL.  Returns
    /// `None` if the ID was unparseable or ambiguous.
    ///
    /// # Errors
    /// This function fails if the asset proxy configured by `args` has an
    /// invalid URL
    #[inline]
    pub fn proxy_url<'a>(
        args: &AssetProxyArgs,
        id: &'a AssetIdentifier,
        query: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Option<Url>> {
        proxy_url_hinted(args, id, None, query)
    }
}

#[cfg(feature = "asset-cdn")]
pub use cdn::*;
