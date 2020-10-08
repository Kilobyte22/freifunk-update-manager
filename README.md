This tool is intended for rolling out mesh-compatibility breaking updates in Freifunk networks.

## Features
* Updates nodes at the bottom of the mesh tree first
* Heuristics for detecting successful updates (taking into account that an updated node will not be able to re-connect until its uplink is updated as well)
* Proper handling of nodes with autoupdates
* Handling of nodes which can't apply updates (for example because no matching upgrade is found)
* Metrics
* History keeping of uplink records for offline nodes

## To be Implemented
* Handling of nodes which have a broken auto-updater, which does not actually request updates

## Requirements
* Needs to run on the server which Freifunk nodes query for updates
* A HTTP accessible version of the meshviewer.json

## Compiling and Installing
You needs a reasonably new version of cargo and rustc. Obtain this using either your package manager or if thats too old using [rustup.rs](https://rustup.rs). (If you get compile errors, you can assume your rustc is too old)

```
cargo build --release
```
The first build might take around 5 minutes, subsequent builds after an update should be considerably faster (on the order of 30 seconds)

After that you can install using `make install`

## Typical Workflow
1. Install gluon-update-manager and adjust its config file. You probably want to set `enabled` to false, which means that it will collect internal state and start logging historic link records, but not serve any upgrades. Alternatively you can also enable dry-run. You then now slowly see routers getting failures (if you look at the stats or logs). This is expected behaviour, as the software is now doing normal operation, except for actually serving the new firmware.
2. Set up your webserver to forward requests for the firmware update to gluon-update-manager. Please ensure that the http path at which the actual firmware resides (old and new) gets directly served by your werbserver. The full request path in the format `/{site}/{branch}/{filename}` must be forwarded. You might also want to expose `/node_dump.json`, which reports internal statistics. An example config for nginx might look like this:  
```
    location /wetter/stable/ {
            proxy_set_header X-Forwarded-For $remote_addr;
            proxy_pass http://[::1]:6060;
    }

    location /wetter/beta/ {
            proxy_set_header X-Forwarded-For $remote_addr;
            proxy_pass http://[::1]:6060;
    }

    location = /node_dump.json {
            proxy_set_header X-Forwarded-For $remote_addr;
            proxy_pass http://[::1]:6060/node_dump.json;
    }
```
3. If you set `enabled` to `false` in Step 1, wait about a week before continuing with the next step.
4. Ensure the firmware is ready to go. Make sure it is in the correct location. For best results, the firmware should be dated a couple of days back, as gluon-auto-updater tends to ignore relatively new updates. This can lead to failed updates and therefore skipped nodes
5. Set dry-run to false and enabled to true. If you had dry-run enabled, when disabling it, make sure to delete (or rename) the `node_info` key in the state json file - to reset the internal state. If you don't need historic link records, you can also just delete the file
6. Closely monitor update progress. The stdout logs of gluon-update-manager as well as the access logs of your webserver are your best friend. The node dump can also be helpful, especially when paired with tools like grafana.