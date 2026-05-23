#! /usr/bin/env bash

TODO=./data/todo
DONE=./data/done

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

  # if the title is like #aze, let's make it # aze
  if [[ ${title:1:1} != " " ]]; then
    title="# ${title:1}"
  fi
   
  id="id: $(uuidgen | cut -c1-8)"
  content="$id
$(cat $note_tmp_file)
---
$(cat $TODO)"

  echo "$content" > "$TODO"

}

function get_todo_notes() {
  readarray -d '' notes < <(
    awk -v RS='---' '
    {
      gsub(/^[ \n]+|[ \n]+$/, "", $0)  # trim

      if ($0 ~ /[^[:space:]]/) {       # only output non-empty blocks
        printf "%s\0", $0
      }
    }
  ' "$TODO"
  )
}

function show_notes() {
  get_todo_notes

  note_tmp_file=$(mktemp)

  for note in "${notes[@]}"; do
    title="${note#*# }"
    echo "[] ${title}" >> "$note_tmp_file"
  done

  alacritty -T "new-note" -e hx "$note_tmp_file"

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



