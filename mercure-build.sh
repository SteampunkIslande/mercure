#!/bin/bash

cargo build --release

singularity build --fakeroot -F mercure-webapp.sif mercure-webapp.def

singularity build --fakeroot -F mercure-routine.sif mercure-routine.def

