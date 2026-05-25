#!/usr/bin/env -S uv run --script
# 
# /// script
# requires-python = ">=3.14"
# dependencies = ["typer", "aiofile", "pyyaml"]
# ///


import typer
import asyncio
import aiofile
import yaml
from pathlib import Path

def main(notes_folder: Path = Path("~/.config/notes/data"), filters: list[str] = []) -> list[str]:
    filters = [f for f in filters if f]
    return asyncio.run(format_tags(notes_folder, filters))

def print_for_fzf(path: Path, data: dict) -> None:
    tags = data.get("tags", [])
    title = data["title"]

    def get_tags_attr(index: int) -> str:
        try:
            return tags[index]
        except KeyError:
            return ""

    if priority := data.get("priority", []):
        print(f"{path.name}\t[{priority}]\t{title}", end="")
    else:
        print(f"{path.name}\t#{get_tags_attr(0)} #{get_tags_attr(1)}\t{title}", end="")
    

async def get_file_metadata(path: Path) -> dict:
    async with aiofile.async_open(path) as stream:
        while await stream.readline() != "---\n":
            continue

        metadata = ""

        while (l := await stream.readline()) != "---\n":
            metadata += str(l)

        data = yaml.load(metadata, yaml.CLoader)

        while (l := await stream.readline()):
            if str(l).startswith("#"):
                data["title"] = str(l)[2:]
                break

        return data

async def parse_note(path: Path, filters: list[str]) -> None:
    data = await get_file_metadata(path)

    tags = data.get("tags", [])

    if filters and any(filter not in tags for filter in filters):
        return

    return print_for_fzf(path, data)
    

async def format_tags(notes_folder: Path, filters: list[str]) -> list[str]:
    tasks = [
        parse_note(Path(root) / file, filters)
        for root, _, files in notes_folder.walk()
        for file in files
    ]

    return await asyncio.gather(*tasks)


if __name__ == "__main__":
    typer.run(main)
