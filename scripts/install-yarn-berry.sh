#!/bin/bash

set -e

yarn set version berry
yarn plugin import interactive-tools
yarn plugin import typescript
yarn plugin import version
