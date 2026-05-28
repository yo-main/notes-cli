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

def get_attr_safe(values: list, index: int) -> str:
    try:
        return values[index]
    except IndexError:
        return ""

def main(notes_folder: Path = Path("~/.config/notes/data"), filters: list[str] = []) -> list[str]:
    filters = [f for f in filters if f]
    return asyncio.run(format_tags(notes_folder, filters))

def print_for_fzf(path: Path, data: dict) -> list[str]:
    tags = data.get("tags", [])
    title: str = data["title"]
    created = data["created"]


    if priority := data.get("priority", []):
        return [path.name,"\t", f"[{created}]", "\t", f"[{priority}]", " ", f"#{get_attr_safe(tags, 0)}", "\t", title]
    else:
        return [path.name,"\t", f"[{created}]", "\t", f"#{get_attr_safe(tags, 0)}", " ", f"#{get_attr_safe(tags, 1)}", "\t", title]
    

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

async def parse_note(path: Path, filters: list[str]) -> list[str]:
    data = await get_file_metadata(path)

    tags = data.get("tags", [])

    if filters:
        for filter in filters:
            match filter[0]:
                case "-":
                    if filter[1:] in tags:
                        return []
                case "+":
                    if filter[1:] not in tags:
                        return []
                case _:
                    if filter not in tags:
                        return []

    return print_for_fzf(path, data)
    

async def format_tags(notes_folder: Path, filters: list[str]) -> None:
    tasks = [
        parse_note(Path(root) / file, filters)
        for root, _, files in notes_folder.walk()
        for file in files
    ]

    data = await asyncio.gather(*tasks)

    data = [d for d in data if d]
    data.sort(key=lambda d: (get_attr_safe(d, 1), get_attr_safe(d, 2)))

    if not data:
        return

    max_lengths = [max(len(row[i]) for row in data) for i in range(len(data[0]))]

    for d in data:
        print("".join(
            d[i].ljust(max_lengths[i])
            for i in range(len(d))
        ).strip())
    


if __name__ == "__main__":
    typer.run(main)
