use crate::error::ContractError;
use cw20::{EmbeddedLogo, Logo};

const LOGO_SIZE_CAP: usize = 5 * 1024;

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

pub fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 12 {
        return false;
    }
    for byte in bytes.iter() {
        if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
            return false;
        }
    }
    true
}

/// Checks if passed logo is correct, and if not, returns an error
pub fn verify_logo(logo: &Logo) -> Result<(), ContractError> {
    match logo {
        Logo::Embedded(EmbeddedLogo::Svg(logo)) => verify_xml_logo(logo),
        Logo::Embedded(EmbeddedLogo::Png(logo)) => verify_png_logo(logo),
        Logo::Url(_) => Ok(()), // Any reasonable url validation would be regex based, probably not worth it
    }
}

/// Checks if data starts with XML preamble
fn verify_xml_preamble(data: &[u8]) -> Result<(), ContractError> {
    // The easiest way to perform this check would be just match on regex, however regex
    // compilation is heavy and probably not worth it.

    let preamble = data
        .split_inclusive(|c| *c == b'>')
        .next()
        .ok_or(ContractError::InvalidXmlPreamble {})?;

    const PREFIX: &[u8] = b"<?xml ";
    const POSTFIX: &[u8] = b"?>";

    if !(preamble.starts_with(PREFIX) && preamble.ends_with(POSTFIX)) {
        Err(ContractError::InvalidXmlPreamble {})
    } else {
        Ok(())
    }

    // Additionally attributes format could be validated as they are well defined, as well as
    // comments presence inside of preable, but it is probably not worth it.
}

/// Validates XML logo
fn verify_xml_logo(logo: &[u8]) -> Result<(), ContractError> {
    verify_xml_preamble(logo)?;

    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::LogoTooBig {})
    } else {
        Ok(())
    }
}

/// Validates png logo
fn verify_png_logo(logo: &[u8]) -> Result<(), ContractError> {
    // PNG header format:
    // 0x89 - magic byte, out of ASCII table to fail on 7-bit systems
    // "PNG" ascii representation
    // [0x0d, 0x0a] - dos style line ending
    // 0x1a - dos control character, stop displaying rest of the file
    // 0x0a - unix style line ending
    const HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::LogoTooBig {})
    } else if !logo.starts_with(&HEADER) {
        Err(ContractError::InvalidPngHeader {})
    } else {
        Ok(())
    }
}
