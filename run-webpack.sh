#!/usr/bin/env bash
cd assets
NODE_ENV=dev npm run webpack "$@"
