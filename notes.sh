#! /usr/bin/env bash

TODO=~/.config/notes/data/todo
DONE=~/.config/notes/data/done

function new_note() {

  id="$(uuidgen | cut -c1-8)"
  filename="$TODO/$id.md"

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
      if [[ "$done_id" == "${filename:-3}" ]]; then
        note_done "$filename"
        break
      fi
    done
  done
}

function open_note() {
  view_id="$1"
  open_file "$TODO/$view_id" +4
}


function todo_format() {
  for path in "$TODO"/*; do
    [[ -f "$path" ]] || continue

    filename="$(basename "$path")"
    priority="$(grep "^priority: " "$path" | cut -d' ' -f2)"
    title="$(grep "^# " "$path" | head -1 | cut -c3-)"
    printf "%s\t[%s] %s\n" "$filename" "$priority" "$title"
  done
}

function list_notes() {
  selected=$(
    todo_format \
      | fzf \
          -m \
          --with-nth=2 \
          --delimiter=$'\t' \
          --preview="glow -p -s dark $TODO/{1}" \
          --preview-window=bottom \
          --prompt="todos> " \
          --bind "enter:execute(notes open-note {1})" \
          --bind "ctrl-o:execute-silent(echo {+1} | xargs -n1 notes done)+reload(notes todo-format)" \
          --bind "ctrl-n:become(notes new)"
          # --preview="bat --color=always $TODO/{1}" \
          # --bind "ctrl-d:execute-silent(echo {+1} | xargs -n1 rm $TODO/)+reload($LIST_CMD)" \
  )

  # mark_note_as_done "$note_tmp_file"
}

function note_done() {
  filename="$1"

  mv "$TODO/$filename" "$DONE/$filename"
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

  done)
    shift 1
    note_done "$@"
    ;;

  "todo-format")
    shift 1
    todo_format "$@"
    ;;

  "open-note")
    shift 1
    open_note "$@"
    ;;

  *)
    echo unknown command
    ;;

esac



