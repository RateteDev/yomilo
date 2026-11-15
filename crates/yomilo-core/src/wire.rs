use crate::{PreparationVersions, Ratio, ValidatedSvg, check_preparation_versions};
use serde::{Deserialize, Deserializer, de::Error};

pub(crate) mod base64_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        STANDARD
            .decode(String::deserialize(deserializer)?)
            .map_err(D::Error::custom)
    }
}

pub(crate) fn deserialize_versions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<PreparationVersions, D::Error> {
    let versions = PreparationVersions::deserialize(deserializer)?;
    check_preparation_versions(versions).map_err(D::Error::custom)?;
    Ok(versions)
}

impl<'de> Deserialize<'de> for Ratio {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Dimensions {
            width: u32,
            height: u32,
        }
        let dimensions = Dimensions::deserialize(deserializer)?;
        if dimensions.width == 0 || dimensions.height == 0 {
            return Err(D::Error::custom("Ratio dimensions must be positive"));
        }
        Ok(Self {
            width: dimensions.width,
            height: dimensions.height,
        })
    }
}

impl<'de> Deserialize<'de> for ValidatedSvg {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct SvgData {
            source: String,
            element_names: Vec<String>,
            attribute_names: Vec<String>,
        }
        let data = SvgData::deserialize(deserializer)?;
        // JSONから検証済み型を偽造できないよう、本文と使用名一覧を再検査する。
        let source = crate::source::Source::new(&data.source);
        let mut diagnostics = crate::diagnostic::DiagnosticBag::default();
        let validated = crate::svg::validate(&source, 0..data.source.len(), &mut diagnostics)
            .map_err(|diagnostic| D::Error::custom(diagnostic.message))?;
        diagnostics.finish().map_err(D::Error::custom)?;
        let validated =
            validated.ok_or_else(|| D::Error::custom("SVG validation produced no document"))?;
        if validated.elements != data.element_names || validated.attributes != data.attribute_names
        {
            return Err(D::Error::custom(
                "SVG vocabulary does not match its validated source",
            ));
        }
        Ok(validated)
    }
}
