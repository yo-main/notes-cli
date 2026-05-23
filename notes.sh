#! /usr/bin/env bash

DATABASE=./database

function new_note() {

  note_tmp_file=$(mktemp)

  template="date: $(date --iso)
priority: medium

#"

  echo "$template" > "$note_tmp_file"

  alacritty -T "new-note" -e hx "$note_tmp_file" +4

  title=$(sed -n "4p" "$note_tmp_file" | tr -d '[:space:]')
  if [[ "$title" == "#" ]]; then
    return
  fi
 
  
  id="id: $(uuidgen | cut -c1-8)"
  content="$id
$(cat $note_tmp_file)
---
$(cat $DATABASE)  "

  echo "$content" > "$DATABASE"

}

function show_notes() {
  
}

case "$1" in

  --help)
    echo help is to be implemented
    ;;

  --new)
    shift 1
    new_note "$@"
    ;;

  --show)
    shift 1
    show_notes "$@"
    ;;

  *)
    echo unknown command
    ;;

esac



