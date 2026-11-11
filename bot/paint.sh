#!/bin/sh
# Paints a text drawing onto the shared board, one pixel per request.
#   bot/paint.sh bot/crab.txt
# Each character is a pixel; "." leaves it alone. The letters pick palette colors.
URL=${URL:-http://127.0.0.1:3000/api/board}
y=0
while IFS= read -r row; do
  x=0
  for c in $(printf '%s' "$row" | fold -w1); do
    case $c in
      k) color=1 ;; r) color=2 ;; o) color=3 ;; y) color=4 ;; w) color=11 ;; *) color= ;;
    esac
    if [ -n "$color" ]; then
      curl -s -X POST "$URL" -H 'content-type: application/json' \
        -d "{\"x\":$x,\"y\":$y,\"color\":$color}"
      sleep 0.02
    fi
    x=$((x + 1))
  done
  y=$((y + 1))
done < "$1"
echo "painted $1"
