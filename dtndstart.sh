#!/bin/bash

dtnd -n $(hostname) -e incoming -C mtcp -p 3s -r epidemic
