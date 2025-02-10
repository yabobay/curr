#!/bin/sh

cargo rustc --release -- -C target-feature=+crt-static
