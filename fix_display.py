import sys

def fix_display(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # Need to change `.to_owned()` into `write!(f, "{}", match ...)` or similar.
    content = content.replace(
        "        match self {\n            KeyName::ArrowLeft => \"ArrowLeft\",\n            KeyName::ArrowRight => \"ArrowRight\",\n            KeyName::ArrowUp => \"ArrowUp\",\n            KeyName::ArrowDown => \"ArrowDown\",\n            KeyName::Ok => \"Ok\",\n        }\n        .to_owned()",
        "        let s = match self {\n            KeyName::ArrowLeft => \"ArrowLeft\",\n            KeyName::ArrowRight => \"ArrowRight\",\n            KeyName::ArrowUp => \"ArrowUp\",\n            KeyName::ArrowDown => \"ArrowDown\",\n            KeyName::Ok => \"Ok\",\n        };\n        write!(f, \"{}\", s)"
    )

    with open(filename, 'w') as f:
        f.write(content)

fix_display("sdk/sender/fcast-sender-sdk/src/device.rs")

def fix_clippy(filename):
    with open(filename, 'r') as f:
        content = f.read()

    # fix empty line after outer attr in lib.rs
    content = content.replace("#[cfg(any_protocol)]\n\nimpl std::fmt::Display for IpAddr", "impl std::fmt::Display for IpAddr")

    # fix single match in lib.rs
    content = content.replace("                match &mut ip {\n                    IpAddr::V6 { scope_id, .. } => *scope_id = this_scope_id,\n                    _ => (),\n                }", "                if let IpAddr::V6 { scope_id, .. } = &mut ip {\n                    *scope_id = this_scope_id;\n                }")

    with open(filename, 'w') as f:
        f.write(content)

fix_clippy("sdk/sender/fcast-sender-sdk/src/lib.rs")

def add_allow(filename):
    with open(filename, 'r') as f:
        content = f.read()

    content = content.replace("    async fn load(", "    #[allow(clippy::too_many_arguments)]\n    async fn load(")
    content = content.replace("    fn load_url(", "    #[allow(clippy::too_many_arguments)]\n    fn load_url(")

    with open(filename, 'w') as f:
        f.write(content)

add_allow("sdk/sender/fcast-sender-sdk/src/fcast.rs")
