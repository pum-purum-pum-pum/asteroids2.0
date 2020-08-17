#!/usr/bin/bash

createsvg() {
  local d
  local svg
  for d in *.ai; do
    svg=$(echo "$d" | sed 's/.ai/.svg/')
    echo "creating $svg ..."
    inkscape -f "$d" -l "$svg"
  done
}

if [ "$1" != "" ];then
  cd $1
fi

createsvg