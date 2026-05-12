import sys

def fix_mcore(filename):
    with open(filename, 'r') as f:
        content = f.read()

    content = content.replace("    pub async fn from_preview(", "    #[allow(clippy::too_many_arguments)]\n    pub async fn from_preview(")

    with open(filename, 'w') as f:
        f.write(content)

fix_mcore("sdk/mirroring_core/src/transmission.rs")
