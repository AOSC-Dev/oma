Notes for Contributors
===

Your contributions are welcome in the form of pull requests. To ease review and, more importantly, for timely reviews of bug-fixes and features, please follow the following guidelines when opening pull requests:

- Follow the [Conventional Commits](https://www.conventionalcommits.org/) to format your commit headings and messages - extra credits for detailed explanation of your proposed changes in the main commit message.
- For bug-fixes and minor changes to interface and behaviors, please submit your patches to the `master` branch for inclusion in the current "major" series.
- For overhauls, refactors, and new features, please submit your patches to the `next` branch for inclusion in the next "major" series.

Branching
---

By and large, oma follows [semantic versioning](https://semver.org/) and uses a three-segment versioning scheme (`x.y.z`):

- X: Major series, for major features, overhauls, and function refactors.
- Y: Minor series, for minor changes to interface to behaviors.
- Z: Patch series, for bugfixes only.

Environment
---

**Pre-Commit Configuration**

Pre-commit is a standard tool for [git hooks](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks)

We recommend that contributors use `pre-commit hooks` to check for linting issues and typos before committing and pushing changes. However, this step is OPTIONAL and can be safely ignored if preferred.

- pre-commit-hooks (Basic Hooks):
1. trailing-whitespace
2. end-of-file-fixer
3. check-yaml
4. check-added-large-files

- rustfmt
- clippy format
- typos


### Usage:

1. Install `pre-commit` from pip or package managers:

***Example***:
```
# For arch user:
pacman -S pre-commit
# For pip user:
pip install -S pre-commit
```
There is no pre-commit package in AOSC repos yet. \
Currently in AOSC OS, pip is for python2, please use pip3 or venv/conda!

2. Install pre-commit hooks:
```
# In the root of repo:
pre-commit install
```

3. Test configuration
```
pre-commit run -a
```

Requesting for Reviews
---

Your pull requests would notify all contributors. If you want to expedite things, consider joining our [chat groups](https://aosc.io/contact) for a good scream. ;-)
