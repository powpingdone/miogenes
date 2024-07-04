#!/usr/bin/python3

import os
import sys
import subprocess as sp
import multiprocessing as mp


class InDir:
    def __init__(self, dir, cmds):
        self.dir = dir
        self.cmds = Target(cmds)

    def run(self):
        where_we_used_to_be = os.getcwd()
        os.chdir(self.dir)
        self.cmds.begin()
        os.chdir(where_we_used_to_be)


class Cmd:
    def __init__(self, cmd):
        self.cmd = cmd

    def run(self):
        sp.run(self.cmd, shell=True, check=True)


class Spawn:
    def __init__(self, cmds):
        self.int_target = Target(cmds)

    def run(self):
        proc = mp.Process(target=self.int_target.begin)
        proc.start()
        return proc


class Target:
    def __init__(self, cmds, desc=""):
        self.cmds = cmds
        self.desc = desc

    def __repr__(self):
        return self.desc

    def begin(self):
        handles = [x.run() for x in self.cmds]
        [x.join() for x in handles if x is not None]


TARGETS = {
    "setup": Target(
        [
            Cmd("cargo install --locked genemichaels sqlx-cli"),
            Cmd("sqlx migrate run --source backend/migrations"),
            Cmd("cargo sqlx prepare --workspace"),
            InDir(
                "frontend",
                [
                    Cmd("flutter precache"),
                ],
            ),
        ],
        desc="Setup the environment for developing/building in",
    ),
    "fmt": Target(
        [
            Cmd("genemichaels"),
            Cmd("cargo fmt"),
            InDir("frontend", [Cmd("dart format .")]),
        ],
        desc="Format all code files",
    ),
    "build": Target(
        [
            InDir(
                "frontend",
                [
                    Cmd("dart run build_runner build --delete-conflicting-outputs"),
                ],
            ),
            Cmd("cargo build"),
            # InDir("frontend", [Cmd("flutter build")]),
        ],
        desc="Build programs in debug mode",
    ),
    "server": Target(
        [
            Cmd("DATA_DIR='./files' IP_ADDR='127.0.0.1' PORT=8081 SIGNUP_ENABLED=1 cargo run -p mio-backend") 
        ],
        desc="Run debug mode server"
    ),
}


def print_targets():
    print("Currently configured targets:")
    for k, v in TARGETS.items():
        print(f"\t{k}: {v.desc}")
    sys.exit(0)


def main():
    if len(sys.argv) < 2:
        print(f"usage: {sys.argv[0]} <targets>")
        print_targets()
    for target in sys.argv[1:]:
        if target not in TARGETS:
            print(f"selected target '{target}' not in list of targets.")
            print_targets()
    for target in sys.argv[1:]:
        TARGETS[target].begin()


if __name__ == "__main__":
    mp.set_start_method("spawn")
    main()
