# Design though

## Git
Move the special action dedicated to git in a git sub-action CLI.
`workspace git status`
`workspace git delete-branch`
`workspace git fetch`
- fetch remote
- delete deleted branches
- automatically rebase branches when its possible

See how to install git-* commands?

Create a git module to split the big git.rs file.
git/mod.rs
    - new_git_command
    - get_remote_url_repo
git/status.rs
git/submodules.rs

## Repositories

List Repositories.
Add Submodules as Element of the Repositories. !!!THIS IS A GREEDY STEP. HOW
TOMAKE IT FASTER!?

Add a HashMap registring the repositories in the workspace globally:

One instance per-repository tracking all the different clone of the same
repository (`main_repo: Option<PathBuf>, submodules: Vec<>`)


## Tree

workspace tree wished output:

/home/hlefevre/work
├── kernel/pub/scm/linux/kernel/git/torvalds/linux-2.6
│   - remote: git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux-2.6.git
│   - submodules:
│       - sub_1_relpath => forge/name => path/to/main.git
│       - sub_2_relpath => forge/name => path/to/main.git
├── gitlab
│   ├── hpaluche
│   │   ├── pyhuebox            -- git@gitlab.com:hpaluche/pyhuebox.git
│   │   ├── pyhue               -- git@gitlab.com:hpaluche/pyhue.git
│   │   ├── workspace           -- git@gitlab.com:hpaluche/workspace.git
│   │   ├── setup               -- git@gitlab.com:hpaluche/setup.git
│   │   └── kavist              -- git@gitlab.com:hpaluche/kavist.git
│   └── AlexisPolti/iot-node    -- git@gitlab.com:AlexisPolti/iot-node.git
├── github
│   ├── evenfurther/pathfinding -- git@github.com:evenfurther/pathfinding.git
│   ├── SiemaApplications
│   │   ├── microcom-ucc        -- https://github.com/SiemaApplications/microcom-ucc.git
│   │   ├── action-mirror       -- https://github.com/SiemaApplications/action-mirror.git
│   │   └── mirror-action       -- https://github.com/SiemaApplications/mirror-action.git
│   ├── phillipberndt/autorandr -- git@github.com:phillipberndt/autorandr.git
│   ├── rikmeijer/googlephotos2nextcloud -- https://github.com/rikmeijer/googlephotos2nextcloud.git
│   ├── Paluche
│   │   ├── esp32-playground    -- git@github.com:Paluche/esp32-playground.git
│   │   ├── smart-rebase        -- git@github.com:Paluche/smart-rebase.git
│   │   ├── Telecom             -- git@github.com:Paluche/Telecom.git
│   │   ├── jj                  -- git@github.com:Paluche/jj.git
│   │   ├── colost              -- git@github.com:Paluche/colost.git
│   │   ├── scratchpad          -- git@github.com:Paluche/scratchpad.git
│   │   ├── git-status          -- git@github.com:Paluche/git-status.git
│   │   ├── stm32l4-discovery   -- git@github.com:Paluche/stm32l4-discovery.git
│   │   ├── jj-prompt           -- git@github.com:Paluche/jj-prompt.git
│   │   ├── advent-of-code      -- git@github.com:Paluche/advent-of-code.git
│   │   └── rc_files            -- git@github.com:Paluche/rc_files.git
│   ├── espressif/esp-idf       -- https://github.com/espressif/esp-idf.git
│   ├── stm32-rs/stm32l4xx-hal  -- git@github.com:stm32-rs/stm32l4xx-hal.git
│   ├── net-snmp/net-snmp       -- git@github.com:net-snmp/net-snmp.git
│   ├── pre-commit/pre-commit   -- git@github.com:pre-commit/pre-commit.git
│   ├── embassy-rs
│   │   ├── nrf-softdevice      -- git@github.com:embassy-rs/nrf-softdevice.git
│   │   ├── stm32-data          -- git@github.com:embassy-rs/stm32-data.git
│   │   └── embassy             -- https://github.com/embassy-rs/embassy.git
│   ├── colored-rs
│   │   └── colored             -- git@github.com:colored-rs/colored.git
│   ├── Hubert-Lefevre_vossloh
│   │   └── git-migration-tools -- https://github.com/Hubert-Lefevre_vossloh/git-migration-tools.git
│   ├── rektdeckard
│   │   └── hues                -- git@github.com:rektdeckard/hues.git
│   └── adi1090x
│       └── rofi                -- https://github.com/adi1090x/rofi.git
├── lefevre-thourault
│   ├── embedded
│   │   ├── modules/ac-protocol -- git@gigabyte.lefevre-thourault.fr:embedded/modules/ac-protocol.git
│   │   └── nis                 -- git@gigabyte.lefevre-thourault.fr:embedded/nis.git
│   └── core
│       ├── ci-tools            -- git@gigabyte.lefevre-thourault.fr:core/ci-tools.git
│       └── python-emb-tools    -- git@gigabyte.lefevre-thourault.fr:core/python-emb-tools.git
├── buildroot                   -- https://git.buildroot.net/buildroot
├── bitbucket/jpaluche/nrf_projects -- git@bitbucket.org:jpaluche/nrf_projects.git
└── local
    ├── nRF5_SDK_17.1.0_ddde560
    ├── rust_test
    ├── backlight
    ├── tp-nrf
    ├── nordic/softdevices
    │   ├── s140
    │   ├── s113
    │   └── s112
    ├── lsm6dsl
    ├── C++-test
    ├── nextcloud-migration
    ├── rebase-workflow
    ├── wm_challenge_rust
    ├── test_jj
    ├── test_colored
    ├── C-test
    ├── tp-rust-2
    └── cube
