use super::policy::Kind;

pub(super) fn expected(kind: Kind) -> String {
    let description = match kind {
        Kind::Enum(values) => return values.replace('|', ", "),
        Kind::Id | Kind::Identifier => {
            "a nonempty name using letters, digits, underscore, hyphen, dot, or colon"
        }
        Kind::IdList => "space-separated local SVG IDs",
        Kind::Href => {
            "#id pointing to a declared ID in this SVG; data and external URLs are not allowed"
        }
        Kind::Url => "none or url(#id) pointing to a declared ID in this SVG",
        Kind::Color => "a CSS named/hex/functional color, currentColor, or var(--token)",
        Kind::Paint => "a color, none, context-fill, context-stroke, or url(#id)",
        Kind::Path => "SVG path commands and numeric coordinates",
        Kind::Transform => {
            "matrix, translate, scale, rotate, skewX, or skewY with numeric arguments"
        }
        Kind::Timing => {
            "clock offsets, indefinite, local SVG events, or local animation ID.begin/ID.end/ID.repeat(n) with optional offsets"
        }
        Kind::Clock => "a nonnegative SMIL clock, media, or indefinite",
        Kind::Repeat => "a positive repeat count or indefinite",
        Kind::KeyTimes => "semicolon-separated numbers between 0 and 1",
        Kind::KeySplines => "semicolon-separated groups of four numbers between 0 and 1",
        Kind::Length | Kind::PositiveLength | Kind::Lengths => {
            "numeric SVG lengths with standard units or percentages"
        }
        Kind::ViewBox => "four numbers with positive width and height",
        Kind::AspectRatio => {
            "none or xMin/xMid/xMax YMin/YMid/YMax alignment with optional meet or slice"
        }
        Kind::FontFamily => {
            "local font-family names; URL functions and CSS escapes are not allowed"
        }
        Kind::FontSize => "a standard font-size keyword or nonnegative length",
        Kind::FontWeight => {
            "normal, bold, bolder, lighter, inherit, or a number between 1 and 1000"
        }
        Kind::Opacity => "a nonnegative number or percentage",
        Kind::Angle | Kind::Orient | Kind::MotionRotate => {
            "an angle or an element-specific auto orientation"
        }
        Kind::Dasharray => "none or nonnegative SVG lengths",
        Kind::Spacing | Kind::BaselineShift => "an SVG length or a spacing/baseline keyword",
        Kind::Clip => "auto or rect() with four lengths or auto values",
        Kind::Text => "plain XML text",
        Kind::Number
        | Kind::Nonnegative
        | Kind::Integer
        | Kind::PositiveInteger
        | Kind::Numbers
        | Kind::PositiveNumbers
        | Kind::Points => "finite numeric values of the element's required type",
        Kind::AttributeName | Kind::AnimatedValue | Kind::AnimatedValues => {
            "a safe drawing animation target and matching values"
        }
    };
    description.to_owned()
}

pub(super) fn valid(kind: Kind, value: &str) -> bool {
    if matches!(kind, Kind::Text) {
        return true;
    }
    if matches!(kind, Kind::Id) {
        return id(value);
    }
    let value = value.trim();
    if value.is_empty() {
        return matches!(
            kind,
            Kind::IdList | Kind::Path | Kind::Points | Kind::Transform
        );
    }
    if value
        .chars()
        .any(|ch| ch.is_control() && !ch.is_ascii_whitespace())
    {
        return false;
    }
    match kind {
        Kind::Id | Kind::Identifier => id(value),
        Kind::IdList => value.split_ascii_whitespace().all(id),
        Kind::Text => true,
        Kind::Number => number(value).is_some(),
        Kind::Nonnegative => number(value).is_some_and(|v| v >= 0.0),
        Kind::Integer => value.parse::<i64>().is_ok(),
        Kind::PositiveInteger => value.parse::<u32>().is_ok_and(|v| v > 0),
        Kind::Length => length(value, false),
        Kind::PositiveLength => length(value, true),
        Kind::Lengths => length_list(value, false),
        Kind::Numbers => numbers(value).is_some(),
        Kind::PositiveNumbers => numbers(value).is_some_and(|vs| vs.iter().all(|v| *v >= 0.0)),
        Kind::Points => numbers(value).is_some_and(|vs| vs.len() >= 2 && vs.len() % 2 == 0),
        Kind::Path => value.bytes().all(|b| {
            b.is_ascii_digit()
                || b.is_ascii_whitespace()
                || b"MmZzLlHhVvCcSsQqTtAaEe.,+-".contains(&b)
        }),
        Kind::Transform => transform(value),
        Kind::ViewBox => {
            numbers(value).is_some_and(|vs| vs.len() == 4 && vs[2] > 0.0 && vs[3] > 0.0)
        }
        Kind::AspectRatio => aspect_ratio(value),
        Kind::Color => color(value),
        Kind::Paint => paint(value),
        Kind::FontFamily => font_family(value),
        Kind::FontSize => {
            matches!(
                value,
                "xx-small"
                    | "x-small"
                    | "small"
                    | "medium"
                    | "large"
                    | "x-large"
                    | "xx-large"
                    | "xxx-large"
                    | "larger"
                    | "smaller"
                    | "inherit"
            ) || length(value, true)
        }
        Kind::FontWeight => {
            matches!(value, "normal" | "bold" | "bolder" | "lighter" | "inherit")
                || number(value).is_some_and(|v| (1.0..=1000.0).contains(&v))
        }
        Kind::Opacity => number(value.strip_suffix('%').unwrap_or(value)).is_some_and(|v| v >= 0.0),
        Kind::Angle => angle(value),
        Kind::Orient => matches!(value, "auto" | "auto-start-reverse") || angle(value),
        Kind::MotionRotate => matches!(value, "auto" | "auto-reverse") || angle(value),
        Kind::Dasharray => value == "none" || length_list(value, true),
        Kind::Spacing => value == "normal" || length(value, false),
        Kind::BaselineShift => {
            matches!(value, "baseline" | "sub" | "super") || length(value, false)
        }
        Kind::Clip => {
            value == "auto"
                || function(value, "rect").is_some_and(|args| {
                    let parts: Vec<_> = args
                        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
                        .filter(|part| !part.is_empty())
                        .collect();
                    parts.len() == 4 && parts.iter().all(|v| *v == "auto" || length(v, false))
                })
        }
        Kind::Href => fragment(value).is_some(),
        Kind::Url => value == "none" || url_fragment(value).is_some(),
        Kind::Enum(values) => values.split('|').any(|v| v == value),
        Kind::Timing => timing(value).is_some(),
        Kind::Clock => matches!(value, "indefinite" | "media") || clock(value).is_some(),
        Kind::Repeat => value == "indefinite" || number(value).is_some_and(|v| v > 0.0),
        Kind::KeyTimes => value
            .split(';')
            .all(|v| number(v.trim()).is_some_and(|v| (0.0..=1.0).contains(&v))),
        Kind::KeySplines => value.split(';').all(|v| {
            numbers(v.trim())
                .is_some_and(|vs| vs.len() == 4 && vs.iter().all(|v| (0.0..=1.0).contains(v)))
        }),
        Kind::AttributeName | Kind::AnimatedValue | Kind::AnimatedValues => false,
    }
}

pub(super) fn id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
}

pub(super) fn fragment(value: &str) -> Option<&str> {
    value.strip_prefix('#').filter(|value| id(value))
}

pub(super) fn url_fragment(value: &str) -> Option<&str> {
    let inner = function(value.trim(), "url")?.trim();
    let inner = if inner.starts_with('\'') {
        inner.strip_prefix('\'')?.strip_suffix('\'')?
    } else if inner.starts_with('"') {
        inner.strip_prefix('"')?.strip_suffix('"')?
    } else {
        inner
    };
    fragment(inner)
}

pub(super) fn references(kind: Kind, value: &str) -> Vec<&str> {
    let value = value.trim();
    match kind {
        Kind::Href => fragment(value).into_iter().collect(),
        Kind::Url | Kind::Paint => url_fragment(value).into_iter().collect(),
        Kind::IdList => value.split_ascii_whitespace().collect(),
        Kind::Timing => timing(value).unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub(super) fn number(value: &str) -> Option<f64> {
    let (number, end) = numeric_prefix(value)?;
    (end == value.len()).then_some(number)
}

fn numeric_prefix(value: &str) -> Option<(f64, usize)> {
    let bytes = value.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut digits = end - start;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        digits += end - start;
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E'))
        && bytes
            .get(end + 1)
            .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-'))
    {
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == start {
            return None;
        }
    }
    value[..end]
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(|v| (v, end))
}

pub(super) fn numbers(mut value: &str) -> Option<Vec<f64>> {
    let mut result = Vec::new();
    while !value.is_empty() {
        value = value.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == ',');
        if value.is_empty() {
            break;
        }
        let (number, end) = numeric_prefix(value)?;
        result.push(number);
        value = &value[end..];
    }
    (!result.is_empty()).then_some(result)
}

fn length(value: &str, nonnegative: bool) -> bool {
    numeric_prefix(value).is_some_and(|(v, end)| {
        (!nonnegative || v >= 0.0)
            && matches!(
                &value[end..],
                "" | "%"
                    | "px"
                    | "em"
                    | "ex"
                    | "ch"
                    | "rem"
                    | "vw"
                    | "vh"
                    | "vmin"
                    | "vmax"
                    | "cm"
                    | "mm"
                    | "Q"
                    | "in"
                    | "pt"
                    | "pc"
            )
    })
}

fn length_list(value: &str, nonnegative: bool) -> bool {
    let parts: Vec<_> = value
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .collect();
    !parts.is_empty() && parts.iter().all(|v| length(v, nonnegative))
}

fn angle(value: &str) -> bool {
    numeric_prefix(value)
        .is_some_and(|(_, end)| matches!(&value[end..], "" | "deg" | "rad" | "grad" | "turn"))
}

pub(super) fn function<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let prefix = value.get(..name.len())?;
    if !prefix.eq_ignore_ascii_case(name) {
        return None;
    }
    value[name.len()..].strip_prefix('(')?.strip_suffix(')')
}

fn color(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if COLORS.contains(&lower.as_str()) {
        return true;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    if let Some(token) = function(value, "var") {
        return token.trim().strip_prefix("--").is_some_and(|token| {
            !token.is_empty()
                && token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        });
    }
    for name in [
        "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch",
    ] {
        if let Some(args) = function(value, name) {
            let parts: Vec<_> = args
                .split(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ',' | '/'))
                .filter(|part| !part.is_empty())
                .collect();
            return matches!(parts.len(), 3 | 4)
                && parts.iter().all(|part| {
                    number(part.strip_suffix('%').unwrap_or(part)).is_some() || angle(part)
                });
        }
    }
    if let Some(args) = function(value, "color") {
        let mut parts = args.split_ascii_whitespace();
        if !matches!(
            parts.next(),
            Some(
                "srgb"
                    | "srgb-linear"
                    | "display-p3"
                    | "a98-rgb"
                    | "prophoto-rgb"
                    | "rec2020"
                    | "xyz"
                    | "xyz-d50"
                    | "xyz-d65"
            )
        ) {
            return false;
        }
        let remaining = parts.collect::<Vec<_>>().join(" ");
        let parts: Vec<_> = remaining
            .split(|ch: char| ch.is_ascii_whitespace() || ch == '/')
            .filter(|part| !part.is_empty())
            .collect();
        return matches!(parts.len(), 3 | 4)
            && parts
                .iter()
                .all(|part| number(part.strip_suffix('%').unwrap_or(part)).is_some());
    }
    false
}

pub(super) fn paint(value: &str) -> bool {
    matches!(value, "none" | "context-fill" | "context-stroke")
        || color(value)
        || url_fragment(value).is_some()
}

fn font_family(value: &str) -> bool {
    let mut quote = None;
    for ch in value.chars() {
        match ch {
            '\'' | '"' if quote == Some(ch) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(ch),
            ch if ch.is_alphanumeric() || matches!(ch, ' ' | '-' | '_' | ',') => {}
            _ => return false,
        }
    }
    quote.is_none()
}

pub(super) fn transform_arguments(name: &str, value: &str) -> bool {
    numbers(value).is_some_and(|vs| match name {
        "matrix" => vs.len() == 6,
        "translate" | "scale" => matches!(vs.len(), 1 | 2),
        "rotate" => matches!(vs.len(), 1 | 3),
        "skewX" | "skewY" => vs.len() == 1,
        _ => false,
    })
}

fn transform(mut value: &str) -> bool {
    if value == "none" {
        return true;
    }
    while !value.trim().is_empty() {
        value = value.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == ',');
        let Some(open) = value.find('(') else {
            return false;
        };
        let Some(close) = value[open + 1..].find(')') else {
            return false;
        };
        if !transform_arguments(value[..open].trim(), &value[open + 1..open + 1 + close]) {
            return false;
        }
        value = &value[open + close + 2..];
    }
    true
}

fn aspect_ratio(value: &str) -> bool {
    let tokens: Vec<_> = value.split_ascii_whitespace().collect();
    let tokens = if tokens.first() == Some(&"defer") {
        &tokens[1..]
    } else {
        &tokens[..]
    };
    match tokens {
        ["none"] => true,
        [align] | [align, "meet" | "slice"] => matches!(
            *align,
            "xMinYMin"
                | "xMidYMin"
                | "xMaxYMin"
                | "xMinYMid"
                | "xMidYMid"
                | "xMaxYMid"
                | "xMinYMax"
                | "xMidYMax"
                | "xMaxYMax"
        ),
        _ => false,
    }
}

pub(super) fn clock(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.contains(':') {
        let parts: Vec<_> = value.split(':').collect();
        let (hours, minutes, seconds) = match parts.as_slice() {
            [minutes, seconds] => (0.0, *minutes, *seconds),
            [hours, minutes, seconds] => (number(hours)?, *minutes, *seconds),
            _ => return None,
        };
        let minutes_number = number(minutes)?;
        let seconds_number = number(seconds)?;
        return (hours >= 0.0
            && minutes.len() == 2
            && (0.0..60.0).contains(&minutes_number)
            && (0.0..60.0).contains(&seconds_number))
        .then_some(hours * 3600.0 + minutes_number * 60.0 + seconds_number);
    }
    let (amount, end) = numeric_prefix(value)?;
    let scale = match &value[end..] {
        "" | "s" => 1.0,
        "ms" => 0.001,
        "min" => 60.0,
        "h" => 3600.0,
        _ => return None,
    };
    (amount >= 0.0)
        .then_some(amount * scale)
        .filter(|v| v.is_finite())
}

fn offset(value: &str) -> bool {
    clock(value.trim().trim_start_matches(['+', '-']).trim()).is_some()
}

pub(super) fn timing_reference(value: &str) -> Option<(&str, &str)> {
    for marker in [".begin", ".end", ".repeat("] {
        if let Some(position) = value.rfind(marker) {
            let id = &value[..position];
            if !self::id(id) {
                continue;
            }
            let mut rest = &value[position + marker.len()..];
            if marker == ".repeat(" {
                let close = rest.find(')')?;
                if rest[..close].parse::<u32>().is_err() {
                    return None;
                }
                rest = &rest[close + 1..];
            }
            let rest = rest.trim();
            if rest.is_empty()
                || matches!(rest.as_bytes().first(), Some(b'+' | b'-')) && offset(rest)
            {
                return Some((id, &value[position..]));
            }
        }
    }
    for event in TIMING_EVENTS {
        if let Some(position) = value.rfind(event)
            && position > 0
            && value.as_bytes()[position - 1] == b'.'
            && id(&value[..position - 1])
            && event_time(&value[position..], event)
        {
            return Some((&value[..position - 1], &value[position - 1..]));
        }
    }
    None
}

pub(super) fn syncbase(value: &str, reference: &str) -> bool {
    value
        .split(';')
        .filter_map(|token| timing_reference(token.trim()))
        .any(|(id, suffix)| {
            id == reference
                && [".begin", ".end", ".repeat("]
                    .iter()
                    .any(|prefix| suffix.starts_with(prefix))
        })
}

fn event_time(value: &str, event: &str) -> bool {
    value.strip_prefix(event).is_some_and(|rest| {
        let rest = rest.trim();
        rest.is_empty() || matches!(rest.as_bytes().first(), Some(b'+' | b'-')) && offset(rest)
    })
}

const TIMING_EVENTS: &[&str] = &[
    "click",
    "dblclick",
    "mousedown",
    "mouseup",
    "mouseover",
    "mousemove",
    "mouseout",
    "mouseenter",
    "mouseleave",
    "pointerdown",
    "pointerup",
    "pointerover",
    "pointerout",
    "pointerenter",
    "pointerleave",
    "pointermove",
    "focusin",
    "focusout",
    "focus",
    "blur",
    "activate",
    "DOMActivate",
    "load",
    "SVGLoad",
    "resize",
    "SVGResize",
    "scroll",
    "SVGScroll",
];

fn timing(value: &str) -> Option<Vec<&str>> {
    let mut references = Vec::new();
    for token in value.split(';').map(str::trim) {
        if token == "indefinite"
            || offset(token)
            || TIMING_EVENTS.iter().any(|event| event_time(token, event))
        {
            continue;
        }
        let (reference, _) = timing_reference(token)?;
        references.push(reference);
    }
    Some(references)
}

const COLORS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
    "transparent",
    "currentcolor",
    "inherit",
    "initial",
    "unset",
    "revert",
];
