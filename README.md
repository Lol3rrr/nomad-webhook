# Nomad-Webhook
A simple Tool to run certain Operations on Nomad when receiving a Webhook

## Motivation
This Project was born to enable me to restart a certain Nomad Job once a new Version of Container Image
has been built. For this I just register a certain Github Webhook to call this Endpoint and then Restart
the Nomad Job

## Environment Variables
* `LOG_MACHINE` can be set to enable a machine readable (json) logging format
* `CONF_FILE` Specifies the Path to the configuration File (default: config.json)

## Configuration File
The Configuration in JSON Form

### Format
```
{
    "name": {
        "image-tag": {
            "RestartJob": {
                "id": "job-name"
            }
        }
    }
}
```

### Github Webhook
Create a Github Webhook pointing to `[domain]/name` and using the `application/json` content type, with the `package` Option