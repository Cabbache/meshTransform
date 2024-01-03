#!/bin/bash

BIN=../target/release/mesh_transform
function render()
{
	$BIN scale 0.55 0.55 0.55 < cow.stl |
	$BIN translate $1 0.3 0 |
	$BIN warp \
	> cow_out$2.stl && \
	python3 render.py cow_out$2.stl && \
	rm cow_out$2.stl
}

CTR=0
for i in $(seq -4 0.07 1)
do
	render $i $CTR
	CTR=$((CTR+1))
done

ffmpeg -i cow_out%d.stl.png cow.gif
