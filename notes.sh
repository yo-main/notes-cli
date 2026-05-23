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

function mark_note_as_done() {
  todo_file="$1"

  done_ids=$(cat "$todo_file" | grep "[x]" | awk '{print $2}')

  get_todo_notes

  todo=""
  done=$(cat "$DONE")

  for note in "${notes[@]}"; do
    note_id=$(echo "$note" | grep "id: ")
    note_id=${note_id:4}
    marked_as_done=0

    for done_id in "${done_ids[@]}"; do
      if [[ "$done_id" == "$note_id" ]]; then
        done="${note}\n---\n${done}"
        marked_as_done=1
        break
      fi
    done

    if [[ "$marked_as_done" == 0 ]]; then
        todo="${note}\n---\n${todo}"
    fi
    
  done

  echo -e "$todo" > "$TODO"
  echo -e "$done" > "$DONE"
}

function show_notes() {
  get_todo_notes

  note_tmp_file=$(mktemp)

  for note in "${notes[@]}"; do
    title=$(echo "$note" | grep "# ")
    note_id=$(echo "$note" | grep "id: ")
    printf "[] %s - %s\n"  "${note_id:4}" "${title:2}" >> "$note_tmp_file"
  done

  alacritty -T "new-note" -e hx "$note_tmp_file"

  mark_note_as_done "$note_tmp_file"
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



