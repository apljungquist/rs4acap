#!/usr/bin/env sh
set -eux

docker build --file $1/Dockerfile --tag rs4a/$1 $1
docker run \
  --interactive \
  --mount type=tmpfs,dst=$(pwd)/target \
  --net=host \
  --rm \
  --tty \
  --volume $(pwd):$(pwd):ro \
  --workdir $(pwd) \
  rs4a/$1
