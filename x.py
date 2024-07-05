#!/usr/bin/python3

import os
import sys
import subprocess as sp
import multiprocessing as mp
from copy import deepcopy

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


class Fn:
    def __init__(self, fn):
        self.fn = fn

    def run(self):
        self.fn()


class Spawn:
    def __init__(self, target):
        self.int_target = target

    def run(self):
        proc = mp.Process(target=self.int_target.begin)
        proc.start()
        return proc


class Exec:
    def __init__(self, exec, args=[]):
        self.exec = exec
        self.args = [exec] + args

    def run(self):
        os.execvp(self.exec, self.args)


class Target:
    def __init__(self, cmds, desc=""):
        self.cmds = cmds
        self.desc = desc

    def __repr__(self):
        return self.desc

    def begin(self):
        handles = [x.run() for x in self.cmds]
        [x.join() for x in handles if x is not None]


def clean_files_dir():
    from glob import iglob

    for finp in iglob("./files/*"):
        if finp.split('/')[-1][0] != '.':
            os.remove(finp)

TARGETS = {
    "setup": Target(
        [
            Cmd("cargo install --locked genemichaels sqlx-cli tokio-console"),
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
        ],
        desc="Build programs in debug mode",
    ),
    "server": Target(
        [
            Cmd("DATA_DIR='./files' IP_ADDR='127.0.0.1' PORT=8081 SIGNUP_ENABLED=1 cargo run -p mio-backend") 
        ],
        desc="Run debug mode server"
    ),
    "client": Target(
        [
            InDir(
                "frontend",
                [
                    Exec("flutter", ['run'])
                ],
            )
        ],
        desc="Run debug mode client"
    ),
    "clean": Target(
        [
            Cmd("cargo clean"),
            Fn(clean_files_dir),
            InDir(
                "frontend",
                [
                    Cmd("flutter clean")
                ]
            )
        ],
        desc="cleanup all build/runtime dirs"
    ),
}


def print_targets():
    print("Currently configured targets:")
    for k, v in TARGETS.items():
        print(f"\t{k}: {v.desc}")
    print("Postfix any target with a '!' to run that target in parallel with the next target.")
    print("For example, running target 'a' and 'b': `a! b`")
    sys.exit(0)


def main():
    if len(sys.argv) < 2:
        print(f"usage: {sys.argv[0]} <targets>")
        print_targets()
    # check for target
    for target in sys.argv[1:]:
        if target[-1] == '!':
            target = deepcopy(target[:-1])
        if target not in TARGETS:
            print(f"selected target '{target}' not in list of targets.")
            print_targets()
    # runner
    queue = []
    for target in sys.argv[1:]:
        if target[-1] == '!':
            queue += [target[:-1]]
            continue
        elif len(queue) != 0:
            Target([Spawn(TARGETS[x]) for x in (queue + [target])]).begin()
            continue
        TARGETS[target].begin()


if __name__ == "__main__":
    mp.set_start_method("spawn")
    main()
