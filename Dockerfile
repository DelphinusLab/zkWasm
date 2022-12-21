FROM nvidia/cuda:12.0.0-base-ubuntu22.04

# Install required packages and setup ssh access
RUN --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends openssh-server sudo cmake curl build-essential git && rm -rf /var/lib/apt/lists/* \
    mkdir /var/run/sshd && \
    /etc/init.d/ssh start && \
    useradd -rm -d /home/zkwasm -s /bin/bash -g root -G sudo -u 1001 zkwasm && \
    echo 'zkwasm:zkwasm' | chpasswd 

USER zkwasm
# Install Rust toolchain 
ENV PATH="/home/zkwasm/.cargo/bin:${PATH}"
RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain nightly -y 

WORKDIR /home/zkwasm
# Support for cloning from github via https for submodules (wasmi) and download zkWasm repo
RUN git config --global url.https://github.com/.insteadOf git@github.com: && \
    git clone --recursive https://github.com/DelphinusLab/zkWasm.git
WORKDIR /home/zkwasm/zkWasm
RUN cargo build --release 
USER root
VOLUME ["/home/zkwasm"]
EXPOSE 22
CMD ["/usr/sbin/sshd", "-D"]
