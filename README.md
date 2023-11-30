# Kiara bootstrapping app

This exists to install a specified version of python on an end-user's computer, install our required packages into that, then start `kiara-tauri` using that python interpreter.

There's lots of nice UX things we could do here, currently it just thinks silently for a minute or 2 then opens up the `kiara-tauri` window.

At the moment I've only made it work for Mac, but the concept should extend for linux/windows too if/when there's demand

It expects the kiara-tauri project to be cloned in the same directory as this one, and you to have built a binary that will exist in `kiara-tauri/src-tauri/target/release/kiara-tauri`. There's a symlink to this file in `/bin`, which is referenced in `src-tauri/tauri.conf.json`. This will be injected into this application as a [sidecar](https://tauri.app/v1/guides/building/sidecar/)

For now, it assumes the end-user has [pixi](https://pixi.sh/) installed, later on we can bundle this as a sidecar too, or switch to using something else to install the python version and packages. Pixi bootstraps a C compiler to make it possible to install python.



https://github.com/pyenv/pyenv/tree/master/plugins/python-build is a way to install a specific version
we need to set PREFIX env var to tell it where to install (~/.kiara-app), and `PYTHON_CONFIGURE_OPTS="--enable-shared"` to get the shared library that pyo3 needs.


https://github.com/prefix-dev/pixi/blob/main/examples/docker-build/Dockerfile install pixi with curl?