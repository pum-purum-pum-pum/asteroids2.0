rm c.pngns*

createpng() {
  local d
  local png
  for d in *.ai; do
    png=$(echo "$d" | sed 's/\.ai/\.png/')
    echo ""
    echo "creating $png ..."
    inkscape -z "$d" -e "$png"
  done
}

if [ "$1" != "" ];then
  cd $1
fi

createpng