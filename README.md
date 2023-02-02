# Nomad-Webhook
A simple Tool to run certain Operations on Nomad when receiving a Webhook

## Motivation
This Project was born to enable me to restart a certain Nomad Job once a new Version of Container Image
has been built. For this I just register a certain Github Webhook to call this Endpoint and then Restart
the Nomad Job