#!/bin/bash

BIN=../../target/release/mesh_transform
function render()
{
	$BIN scale 0.5,0.5,0.5 < cow.stl |
	$BIN translate $1,0.3,0 |
	$BIN warp \
	--line "-1,0,0 0,1,0" \
	--line "1,1,0 0,0,1" \
	> cow_out$2.stl && \
	python3 render.py cow_out$2.stl && \
	rm cow_out$2.stl
}

CTR=0
for i in $(seq -6 0.06 2)
do
	render $i $CTR
	CTR=$((CTR+1))
done

ffmpeg -i cow_out%d.stl.png cow.gif
