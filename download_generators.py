# /// script
# requires-python = ">=3.13"
# ///
import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser(description="Download Perchance generators")
parser.add_argument("cf_clearance", help="Cloudflare clearance cookie value")
parser.add_argument(
    "-f",
    "--force",
    action="store_true",
    help="Force download even if generator already exists",
)
args = parser.parse_args()

cf_clearance = args.cf_clearance
force = args.force
generator_names = [
    "abstract-noun",
    "animal",
    "archaic-word",
    "body-part",
    "color-name",
    "color",
    "common-noun",
    "concrete-noun",
    "country",
    "dinosaur",
    "emotion",
    "endangered-animal",
    "english-town-name",
    "fruit",
    "greek-god",
    "greek-monster",
    "greek-titan",
    "ingredient",
    "land-animal",
    "monster-edited",
    "monster-type",
    "nationality",
    "nautical-term",
    "netflix-category",
    "noun",
    "object",
    "occupation",
    "person-adjective",
    "planet-name",
    "room-type",
    "sci-fi-noun",
    "terrain",
    "type-of-art-edited",
    "uncountable-noun",
    "unusual-animal",
    "vegetable",
    "venue",
]

# Create generators directory if it doesn't exist
generators_dir = Path("src/builtin_generators")
generators_dir.mkdir(parents=True, exist_ok=True)

# Filter out generators that already exist (unless force is set)
if not force:
    generators_to_download = [
        name
        for name in generator_names
        if not (generators_dir / f"{name}.perchance").exists()
    ]
    skipped = len(generator_names) - len(generators_to_download)
    if skipped > 0:
        print(
            f"Skipping {skipped} generator(s) that already exist (use -f/--force to override)"
        )
    generator_names = generators_to_download

if not generator_names:
    print("All generators already exist. Use -f/--force to re-download them.")
    sys.exit(0)

# Make the POST request using curl (since it bypasses Cloudflare challenges)
url = "https://perchance.org/api/getGeneratorsAndDependencies"
payload_json = json.dumps(
    {"generatorNames": generator_names, "generatorNameToLastKnownEditTime": {}}
)

# Build curl command
curl_cmd = [
    "curl",
    url,
    "--compressed",
    "-X",
    "POST",
    "-H",
    "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
    "-H",
    "Accept: */*",
    "-H",
    "Accept-Language: en-US,en;q=0.5",
    "-H",
    "Accept-Encoding: gzip, deflate, br, zstd",
    "-H",
    "Referer: https://perchance.org/",
    "-H",
    "Content-Type: application/json",
    "-H",
    "Origin: https://perchance.org",
    "-H",
    "Connection: keep-alive",
    "-H",
    f"Cookie: cf_clearance={cf_clearance}",
    "-H",
    "Sec-Fetch-Dest: empty",
    "-H",
    "Sec-Fetch-Mode: cors",
    "-H",
    "Sec-Fetch-Site: same-origin",
    "-H",
    "Priority: u=4",
    "-H",
    "TE: trailers",
    "--data-raw",
    payload_json,
]

# Execute curl and get response
# Use UTF-8 encoding explicitly to avoid Windows cp1252 encoding issues
try:
    result = subprocess.run(
        curl_cmd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    )
    response_text = result.stdout
except subprocess.CalledProcessError as e:
    print(f"Error: curl failed with exit code {e.returncode}")
    if e.stderr:
        print(f"stderr: {e.stderr}")
    if e.stdout:
        print(f"stdout: {e.stdout[:500]}")
    sys.exit(1)

# Process the JSON response (equivalent to jq '.generators | map({"key": .name, "value": .modelText}) | from_entries')
data = json.loads(response_text)
generators = data.get("generators", [])

# Convert to Record<string, string> format
generator_dict = {}
for gen in generators:
    name = gen.get("name")
    model_text = gen.get("modelText", "")
    if name:
        generator_dict[name] = model_text

# Get the current UTC timestamp in ISO8601 format
fetch_timestamp = datetime.now(timezone.utc).isoformat()

# Save each generator to a file
for key, value in generator_dict.items():
    file_path = generators_dir / f"{key}.perchance"
    generator_url = f"https://perchance.org/{key}"
    with open(file_path, "w", encoding="utf-8") as f:
        f.write(f"// {generator_url}\n")
        f.write(f"// Fetched at: {fetch_timestamp}\n")
        f.write("\n")
        f.write(value)
    print(f"Saved {file_path}")

# Generate mod.rs file
# Include all generators that exist on disk, not just the ones downloaded
mod_rs_path = generators_dir / "mod.rs"
existing_generators = sorted(
    [path.stem for path in generators_dir.glob("*.perchance") if path.is_file()]
)
with open(mod_rs_path, "w", encoding="utf-8") as f:
    f.write("pub const GENERATORS: &[(&str, &str)] = &[\n")
    for key in existing_generators:
        f.write(f'    ("{key}", include_str!("{key}.perchance")),\n')
    f.write("];\n")

# Run rustfmt on the mod.rs file
subprocess.run(["rustfmt", mod_rs_path], check=True)

print(f"Generated {mod_rs_path}")
