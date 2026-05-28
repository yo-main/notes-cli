#! /usr/bin/env bash

BASE_FOLDER="$HOME/.config/notes"
CONFIG="$BASE_FOLDER/config.json"
NOTES_FOLDER="$BASE_FOLDER/data"


function new_note() {
  tags="$@"

  is_todo=$(contains "todo" $tags)

  id="$(uuidgen | cut -c1-8)"
  filename="$NOTES_FOLDER/$id.md"

  template="---
created: $(date --iso)"

  if [[ "$is_todo" == 1 ]]; then
    template="${template}
priority: medium"
  fi

  template="${template}
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

  open_file "$filename" +100

  title=$(tail -n 1 "$filename" | tr -d '[:space:]')
  if [[ "$title" == "#" ]]; then
    rm "$filename"
    return
  fi
}


function open_note() {
  view_id="$1"
  shift
  open_file "$NOTES_FOLDER/$view_id" $@
}

function clone_note() {
  view_id="$1"

  id="$(uuidgen | cut -c1-8)"
  cp "$NOTES_FOLDER/$view_id" "$NOTES_FOLDER/$id.md"

  open_file "$NOTES_FOLDER/$id.md" +100
}

function contains() {
  target="$1"
  shift
  values="$@"

  for value in ${values[@]}; do
    if [[ "${value,,}" == "${target,,}" ]]; then
      echo 1
      return
    fi
  done

  echo 0
}

 
function list_notes() {
  tag="$@"

  selected=$(
    ./format_notes.py --notes-folder "$NOTES_FOLDER" --filters "$tag" \
      | fzf \
          -m \
          --ansi \
          --with-nth=2.. \
          --highlight-line \
          --gap=1 \
          --tac \
          --delimiter=$'\t' \
          --preview="glow -s dark $NOTES_FOLDER/{1}" \
          --preview-window='bottom,border-top,~3' \
          --prompt="notes> " \
          --bind "enter:execute-silent(notes open-note {1})+refresh-preview" \
          --bind "ctrl-space:execute-silent(echo {+1} | xargs -n1 notes done)+reload(notes todo-format ${tag})" \
          --bind "ctrl-n:execute-silent(notes new)+reload(notes todo-format ${tag})" \
          --bind "ctrl-b:execute-silent(notes clone-note {1})+reload(notes todo-format ${tag})" \
          --footer "ctrl+space: mark as done - ctrl+n: new note - enter: open note - ctrl-enter: clone a note"
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
  alacritty -T "new-note" -e hx --config ~/.config/notes/helix.config.toml "$file" ${@:-}
  
}

function sync_git() {
  jj --repository "$BASE_FOLDER" st &> /dev/null || return

  branch=$(cat "$CONFIG" | jq -r '.git_branch // "main"')

  diff=$(jj --repository "$BASE_FOLDER" diff)

  if [ -n "$diff" ]; then
    jj --repository "$BASE_FOLDER" git fetch
    jj --repository "$BASE_FOLDER" rebase -d ${branch}@origin

    commit_msg="$(date +'%Y-%m-%d') - to describe"
    jj --repository "$BASE_FOLDER" commit -m "${commit_msg}"

    jj --repository "$BASE_FOLDER" b m ${branch} --to @-
    jj --repository "$BASE_FOLDER" git push -b ${branch}
  fi
}

sync_git &>/dev/null & disown

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
    ./format_notes.py --notes-folder "$NOTES_FOLDER" --filters "${@:-}"
    ;;

  "open-note")
    shift 1
    open_note "$@" 
    ;;

  "clone-note")
    shift 1
    clone_note "$@"
    ;;

  "sync")
    shift 1
    sync_git 
    ;;

  *)
    echo unknown command
    ;;

esac


