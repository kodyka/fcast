import sys

def fix_clippy2(filename):
    with open(filename, 'r') as f:
        content = f.read()

    content = content.replace("#[allow(dead_code)]\n\n#[cfg_attr(feature = \"uniffi\", uniffi::export)]", "#[allow(dead_code)]\n#[cfg_attr(feature = \"uniffi\", uniffi::export)]")

    with open(filename, 'w') as f:
        f.write(content)

fix_clippy2("sdk/sender/fcast-sender-sdk/src/lib.rs")
