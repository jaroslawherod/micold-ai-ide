# The Kubernetes real-cluster suite's SSH image (feature 686, FR-021, research K12): the basic image
# (`packaging/basic`, feature 806) plus npm pointed at the suite's own package source, so the AI CLI
# install the daemon start runs reaches a stand-in `claude` and never the public registry.
#
# Built by `crates/micold-client/tests/kube_real_support` on top of `micold-basic:dev` and loaded
# into the kind cluster with `kind load docker-image`; never published. The source is served by the
# test process on the host side of the docker network `kind`, whose gateway the harness fixes at
# 172.30.0.1 (`MICOLD_KUBE_TEST_HOST_IP`), so a baked address is stable.
ARG BASE=micold-basic:dev
FROM ${BASE}

# npm's builtin config (beside npm itself) is read whatever `--prefix` the install script passes,
# which a global or user `.npmrc` is not.
RUN printf 'registry=http://172.30.0.1:4873/\n' > /usr/local/lib/node_modules/npm/npmrc
