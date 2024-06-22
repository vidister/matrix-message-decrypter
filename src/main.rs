use base64::Engine;
use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of the decrypted E2E Room key export
    #[arg(short, long)]
    keyfile: String,

    /// Path of the JSON file containing the messages
    #[arg(short, long)]
    messagefile: String,

    /// Path to write the output JSON file (default: stdout)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();

    let keyfile_data = fs::read_to_string(args.keyfile).expect("Unable to read keyfile");
    let mut messagefile_reader =
        std::io::BufReader::new(fs::File::open(args.messagefile).expect("Unable to read messagefile"));
    let sessionkeys = get_sessionkeys_from_json(keyfile_data);

    let messages_stream = serde_json::Deserializer::from_reader(&mut messagefile_reader).into_iter().map(Result::unwrap);

    let mut output_file = args.output.map(|output| {
        std::io::BufWriter::new(std::fs::File::create(output).expect("Failed creating output file"))
    });

    for decrypted_message in get_decrypted_messages(messages_stream, sessionkeys) {
        let serialized = serde_json::to_string(&decrypted_message).expect("Failed serlializing finished data to json");

        if let Some(ref mut out) = output_file {
            out
                .write_all(serialized.as_bytes())
                .expect("Failed writing to output file");
            out
                .write_all(b"\n")
                .expect("Failed writing to output file");
            } else {
                println!("{}", serialized);
        }
    }
}

fn get_decrypted_messages(
    messages: impl Iterator<Item = HashMap<String, Value>>,
    sessionkeys: HashMap<String, String>,
) -> impl Iterator<Item = HashMap<String, Value>> {
    messages.map(move |mut m| {
        let message_id = m["event_id"].as_str().unwrap();
        let j: HashMap<String, Value> = serde_json::from_str(&(m["json"].as_str().unwrap()))
            .expect(&format!("Error parsing message {message_id}"));

        let content = j["content"].as_object().unwrap();

        if content.contains_key("session_id") && content.contains_key("ciphertext") {
            let session_id = content["session_id"].as_str().unwrap();
            if sessionkeys.contains_key(session_id) {
                eprintln!("Message {message_id}: Decrypting using key {session_id}");

                // we have to disable padding in the base64 decoder
                let sessionkey = base64::engine::general_purpose::STANDARD_NO_PAD
                    .decode(&sessionkeys[session_id])
                    .unwrap();
                let message = base64::engine::general_purpose::STANDARD_NO_PAD
                    .decode(&content["ciphertext"].as_str().unwrap())
                    .unwrap();
                if let Some(decrypted_message) = get_decrypted_ciphertext(sessionkey, message) {
                    m.insert("content_decrypted".to_string(), serde_json::from_str(&decrypted_message).expect("Parsing decrypted message failed"));
                } else {
                    eprintln!("Message {message_id}: Decrypting message failed unexpectedly");
                }
            } else {
                eprintln!("Message {message_id}: No matching key found, skipping");
            }
        } else {
            eprintln!("Message {message_id}: No encrypted payload, skipping");
        }
        m
    })
}

fn get_sessionkeys_from_json(keys_raw: String) -> HashMap<String, String> {
    eprintln!("Loading Sessionkeys");

    let keys: Vec<HashMap<String, Value>> =
        serde_json::from_str(&keys_raw).expect("Parsing keyfile failed: invalid format");
    keys.iter().fold(HashMap::new(), |mut m, i| {
        m.insert(
            i["session_id"].as_str().unwrap().to_string(),
            i["session_key"].as_str().unwrap().to_string(),
        );
        m
    })
}

fn get_decrypted_ciphertext(sessionkey: Vec<u8>, ciphertext: Vec<u8>) -> Option<String> {
    let session_key = vodozemac::megolm::ExportedSessionKey::from_bytes(&sessionkey).unwrap();
    let mut session = vodozemac::megolm::InboundGroupSession::import(
        &session_key,
        vodozemac::megolm::SessionConfig::version_1(),
    );

    let decrypted = session
        .decrypt(&vodozemac::megolm::MegolmMessage::from_bytes(&ciphertext).unwrap())
        .ok()?;

    Some(String::from_utf8(decrypted.plaintext).unwrap())
}
