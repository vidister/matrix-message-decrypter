# Matrix Message Decrypter

Decrypt Matrix Megolm messages using E2E key backups

## What?

It's a small Rust utility that allows you to use your E2E Room backup keys to decrypt Megolm encrypted messages sent via the Matrix protocol.

There are some special use cases for this tool, let's say you want to recover messages from your homeserver that went potatoes, or you need to access old messages from a blocked user.
Normally you don't need this, except for when things went wrong.

If you work in law enforcement: stop reading and find a better job.

This tool is my first Rust project and I wrote it in two evenings, there is no proper error handling, etc., be warned. I don't think it will eat your homework, but if it does it's your own fault.

## Credits

The [vodozemac](https://github.com/matrix-org/vodozemac) crate does all the heavy lifting here. Thanks!

## Usage
```
Usage: matrix-message-decrypter [OPTIONS] --keyfile <KEYFILE> --messagefile <MESSAGEFILE>

Options:
  -k, --keyfile <KEYFILE>          Path of the decrypted E2E Room key export
  -m, --messagefile <MESSAGEFILE>  Path of the json file containing the messages
  -o, --output <OUTPUT>            Path to write the output json file (default: stdout)
  -h, --help                       Print help
  -V, --version                    Print version
```

Hint when working on huge piles of messages:
The tool loads the whole message file into memory and parses it there. Split your files, implement stream processing or something, idk, you'll figure it out.

## User Guide

We need to perform multiple steps to decrypt our messages:
1. Gather the messages we want to decrypt from the Homeserver database
2. Back up session keys
3. Decrypt session keys
4. Use this tool to decrypt messages

### Create Key Backup

Element Desktop:
1. Click on your profile picture
2. Security and Privacy
3. Cryptography -> Export E2E room keys
4. Set a passphrase and save the file

### Decrypt Key Backup

There's a handy [Python tool to decrypt the key backup](https://github.com/cyphar/matrix-utils/)
It needs the Python package `PyCryptodome`. You'll figure it how to get this to work.

Decrypt your E2E key file using the passphrase you just set:
```bash
# python megolm_backup.py --from element-keys.txt > element-keys-decrypted.json
Backup passphrase [mode=decrypt]:
```

### Export Encrypted Messages

First we need to gather a dump of/ all messages.
In this example we use the JSON export feature of Postgres to export events from the Synapse database.

Export all events:
`psql -d matrix-synapse -qAtX -c "select json_agg(t) FROM (SELECT * from event_json) t;" -o messages.json`

Export all events from specific room:
`psql -d matrix-synapse -qAtX -c "select json_agg(t) FROM (SELECT * from event_json WHERE room_id = '!dfKadcascAbtdeeJdb:example.com') t;" -o messages.json`

### Using this tool

Clone and build:
```bash
git clone https://github.com/vidister/matrix-message-decrypter
cd matrix-message-decrypter
cargo build --release
```

Usage example:
```bash
# ./target/release/matrix-message-decrypter --keyfile keys.json --messagefile messages.json --output messages_decrypted.json

Loading Sessionkeys
Message $pcUildviZA99Sg-RwSaEhVoZzWAjHmdg_u2Dgo7R1yG: Decrypting using key QE9ZaUEayIlJ+V7FPAqvGUlyuSE4MYw+HOvXEZCBOhk
Message $bq1td7Onf2bFCfk3VdYuQS8PHnu5RqAwpwU4y5Het-0: No encrypted payload, skipping
Message $Rbci30NE6oOdhzly-G0Px8XVHwp9QeadZuY98NJu0IM: No encrypted payload, skipping
Message $aAWJHX2QRO_HVBFQiAfe7dQvdWZrCGdOwOFRFkFenIk: Decrypting using key Gl4Bk49rdv+u691gAJlaDlPdYnwIaY+q69MHn17qUpg
Message $kBaxX3l6ctnCDiu6NPdDNNsEgaDkSC6yPb-1-Y3dLqg: No encrypted payload, skipping
[...]
```

Decrypted messages in the output file have a new entry `content_decrypted`.
Example:
```json
  {
    "content_decrypted": {
      "content": {
        "body": "This is the decrypted message body",
        "msgtype": "m.text"
      },
      "room_id": "!eCCElDaZUXYYYMJTl:example.com",
      "type": "m.room.message"
    },
    "internal_metadata": "{}",
    "format_version": 3,
    "event_id": "$pcUildviZA99Sg-RwSaEhVoZzWAjHmdg_u2Dgo7R1yG",
    "json": [...],
    "room_id": "!eCCmkElZUXYYYMJTl:example.com"
  },
```

You can use [jq](https://jqlang.github.io/jq/) to remove clutter:
```bash
jq '[map(select(.content_decrypted))[] | { meta: .json|fromjson|pick(.room_id,.sender,.origin_server_ts), body: .content_decrypted.content.body }] | sort_by(.meta.origin_server_ts)' messages_decrypted.json

[
  {
    "meta": {
      "room_id": "!eCCElDaZUXYYYMJTl:example.com",
      "sender": "@bob:example.com",
      "origin_server_ts": 1639166427121
    },
    "body": "This is the decrypted message body"
  }
]
```

## License

This software is licensed under the GNU General Public License version 3 or later.
See `LICENSE.md`.
