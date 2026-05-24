#! /usr/bin/env bash

NOTES_FOLDER=~/.config/notes/data/

function new_note() {
  tags="$@"

  id="$(uuidgen | cut -c1-8)"
  filename="$NOTES_FOLDER/$id.md"

  template="---
created: $(date --iso)
priority: medium
tags:"

  if [[ -n "$tags" ]]; then
    for tag in "${tags[@]}"; do
      template="${template}
  - ${tag}"
    done
  fi

  template="${template}
---

#"

  echo "$template" > "$filename"

  open_file "$filename" +4

  title=$(tail -n 1 "$filename" | tr -d '[:space:]')
  if [[ "$title" == "#" ]]; then
    rm "$filename"
    return
  fi
}


function open_note() {
  view_id="$1"
  open_file "$NOTES_FOLDER/$view_id" +4
}



function todo_format() {
  tag_to_filter="$1"

  for path in "$NOTES_FOLDER"/*; do
    [[ -f "$path" ]] || continue

    filename="$(basename "$path")"
    file_content=$(cat "$path")
    print=0

    if [[ -z "$tag_to_filter" ]]; then
      print=1
    else
      mapfile -t tags < <(echo "$file_content" | grep -A10 "^tags:" | grep "\- " | sed "s/\- //" | tr -d ' ')
        
      for tag in "${tags[@]}"; do
        if [[ "$tag" == "$tag_to_filter" ]]; then
          print=1
        fi
      done
    fi

    if [[ "$print" == 0 ]]; then
      continue
    fi

    priority="$(echo "$file_content" | grep "^priority: " | cut -d' ' -f2)"
    title="$(echo "$file_content" | grep "^# " | head -1 | cut -c3-)"

    # tag_display=$(printf "#%s " "${tags[@]}")
    printf "%s\t[%s] %s \n" "$filename" "$priority" "$title"

  done
}

function list_notes() {
  tag="$1"

  selected=$(
    todo_format "$tag" \
      | fzf \
          -m \
          --with-nth=2.. \
          --delimiter=$'\t' \
          --preview="glow -s dark $NOTES_FOLDER/{1}" \
          --preview-window=bottom \
          --prompt="todos> " \
          --bind "enter:execute(notes open-note {1})" \
          --bind "ctrl-space:execute-silent(echo {+1} | xargs -n1 notes done)+reload(notes todo-format ${tag})" \
          --bind "ctrl-n:become(notes new)" \
          # --preview="bat --color=always $NOTES_FOLDER/{1}" \
          # --bind "ctrl-d:execute-silent(echo {+1} | xargs -n1 rm $NOTES_FOLDER/)+reload($LIST_CMD)" \
  )
}

function note_done() {
  filename="$1"

  sed -i "s/- todo$/- done/g" "$NOTES_FOLDER/$filename"
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



