# Secret Share

Frontend and backend to share a secrets. Most work is done in the frontend so that the server doesnt know about the contents.

## Flow

### Save

| Client                                     | Server                                          |
| ------------------------------------------ | ----------------------------------------------- |
| Input $message                             |                                                 |
| generate $secret                           |                                                 |
| $enc_message := encrypt($message, $secret) |                                                 |
| $id := hash($enc_message)                  |                                                 |
| Send $id, $secret                          | Receive $id, $secret                            |
|                                            | $enc_secret := encrypt($secret, $master_secret) |
|                                            | put($id, $enc_secret, TIMEOUT)                  |
|                                            | Return OK                                       |
| $url = generate_url($id, $enc_message)     |                                                 |
| Share $url                                 |                                                 |
|                                            |                                                 |

$url has the $enc_message in the anchor url part, so the server doesnt know about it.

### Load

| Client                                     | Server                                         |
| ------------------------------------------ | ---------------------------------------------- |
| Receive $url                               |                                                |
| Extract $enc_message from $url             |                                                |
| $id := hash($enc_message)                  |                                                |
| $secret := fetch($id)                      | Receive $id                                    |
|                                            | $enc_secret = lookup($id)                      |
|                                            | delete($id)                                    |
|                                            | $secret = decrypt($enc_secret, $master_secret) |
|                                            | Return $secret                                 |
| $message := decrypt($enc_message, $secret) |                                                |
|                                            |                                                |
