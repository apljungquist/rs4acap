ARG TAG
FROM --platform=linux/amd64 axisecp/acap-native-sdk:${TAG}
RUN echo '. /opt/axis/acapsdk/environment-setup-*' >> /root/.bashrc
RUN apt-get update \
 && apt-get install --no-install-recommends --yes gcc libc6-dev make \
 && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --no-modify-path --default-toolchain 1.85.1
ENV PATH=/root/.cargo/bin:$PATH
