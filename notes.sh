#! /usr/bin/env bash

TODO=./data/todo
DONE=./data/done

function new_note() {

  id="$(uuidgen | cut -c1-8)"
  filename="$TODO/$id"

  template="date: $(date --iso)
priority: medium

#"

  echo "$template" > "$filename"

  open_file "$filename" +4

  title=$(sed -n "4p" "$filename" | tr -d '[:space:]')
  if [[ "$title" == "#" ]]; then
    rm "$filename"
    return
  fi
}


function mark_note_as_done() {
  todo_file="$1"

  done_ids=$(cat "$todo_file" | grep "[x]" | awk '{print $2}')

  todos=$(ls "$TODO")

  todo=""

  for filename in "${todos[@]}"; do
    for done_id in "${done_ids[@]}"; do
      if [[ "$done_id" == "$filename" ]]; then
        mv "$TODO/$filename" "$DONE/$filename"
        break
      fi
    done
  done
}

function view_note() {
  view_id="$1"

  todos=$(ls "$TODO")

  for filename in "${todos[@]}"; do
    if [[ "${filename}" == "$view_id" ]]; then
      echo -e $(cat "$TODO/$filename")
      break
    fi
  done
}

function open_note() {
  view_id="$1"
  open_file "$TODO/$view_id" +4
}

function list_notes() {
  todos=$(ls "$TODO")

  note_tmp_file=$(mktemp)

  for filename in "${todos[@]}"; do
    if [[ -z "$filename" ]]; then
      echo "No notes"
      return
    fi

    title=$(cat "$TODO/$filename" | grep "# ")
    printf "[] %s - %s\n"  "${filename}" "${title:2}" >> "$note_tmp_file"
  done

  open_file "$note_tmp_file"

  mark_note_as_done "$note_tmp_file"
}


function open_file() {
  file="$1"
  shift
  alacritty -T "new-note" -e hx --config ~/.config/notes/helix.config.toml "$file" "$@"
  
}

case "$1" in

  help)
    echo help is to be implemented
    ;;

  new)
    shift 1
    new_note "$@"
    ;;

  list)
    shift 1
    list_notes "$@"
    ;;

  view)
    shift 1
    view_note "$@"
    ;;

  "open-note")
    shift 1
    open_note "$@"
    ;;

  *)
    echo unknown command
    ;;

esac



