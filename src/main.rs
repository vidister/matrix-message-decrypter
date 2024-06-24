use base64::Engine;
use clap::Parser;
use serde::de;
use serde_json::{json, Value};
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
    let messagefile_data =
        fs::read_to_string(args.messagefile).expect("Unable to read messagefile");
    let sessionkeys = get_sessionkeys_from_json(keyfile_data);
    let messages = get_messages_from_json(messagefile_data);

    let decrypted_messages = serde_json::to_string(&get_decrypted_messages(messages, sessionkeys))
        .expect("Failed serlializing finished data to json");

    if args.output.is_some() {
        let mut file =
            std::fs::File::create(args.output.unwrap()).expect("Failed creating output file");
        let _ = file
            .write_all(decrypted_messages.as_bytes())
            .expect("Failed writing to output file");
    } else {
        println!("{}", decrypted_messages);
    }
}

fn get_decrypted_messages(
    mut messages: Vec<HashMap<String, Value>>,
    sessionkeys: HashMap<String, String>,
) -> Vec<HashMap<String, Value>> {
    for m in messages.iter_mut() {
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

                let decrypted_ciphertext = get_decrypted_ciphertext(sessionkey, message);
                if decrypted_ciphertext.is_err() {
                    let error_message = format!("** Unable to decrypt: DecryptionError: {:?} **", decrypted_ciphertext.unwrap_err());
                    eprintln!("{}", error_message);

                    m.insert("content_decrypted".to_string(), json!({ "msgtype": "m.bad.encrypted", "body": error_message }));
                } else {
                    let decrypted_message = serde_json::from_str(&decrypted_ciphertext.unwrap())
                        .expect("Parsing decrypted message failed");

                    m.insert("content_decrypted".to_string(), decrypted_message);
                }
            } else {
                eprintln!("Message {message_id}: No matching key found, skipping");
            }
        } else {
            eprintln!("Message {message_id}: No encrypted payload, skipping");
        }
    }

    messages
}

fn get_messages_from_json(messages: String) -> Vec<HashMap<String, Value>> {
    serde_json::from_str(&messages).unwrap()
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

fn get_decrypted_ciphertext(sessionkey: Vec<u8>, ciphertext: Vec<u8>) -> Result<String, vodozemac::megolm::DecryptionError> {
    let session_key = vodozemac::megolm::ExportedSessionKey::from_bytes(&sessionkey).unwrap();
    let mut session = vodozemac::megolm::InboundGroupSession::import(
        &session_key,
        vodozemac::megolm::SessionConfig::version_1(),
    );

    session
        .decrypt(&vodozemac::megolm::MegolmMessage::from_bytes(&ciphertext).unwrap())
        .map(|decrypted| String::from_utf8(decrypted.plaintext).unwrap())
}
