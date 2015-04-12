
# latest [![Build Status](https://travis-ci.org/mitchmindtree/latest.svg?branch=master)](https://travis-ci.org/mitchmindtree/latest)

A module for a channel that acts exactly as std::sync::mpsc::channel does, but rather than storing messages in an underlyhing queue, it only stores the latest message.
