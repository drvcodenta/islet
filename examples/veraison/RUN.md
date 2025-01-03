# Introduction

The process consists of several parts:

* provisioning
* gathering measurements
* feeding measurements to veraison and realm verifier
* running realm for verification purposes
* running verification services (veraison/realm verifier)
* verification itself

It is best and even sometimes required that all the required repos are placed in
one directory. I'll call it `CCA` and it will be referred throughout this file.

The following repos will be used:

* Islet: https://github.com/islet-project/islet that provides:

    * the whole SW/FW stack and scripts for running the emulated environment under the FVP
    * Islet HES https://github.com/islet-project/islet/tree/main/hes
    * kvmtool-rim-measurer from https://github.com/islet-project/islet/tree/main/third-party/

* Various miscelanous tools and libraries for Remote Attestation

    * rust-rsi: https://github.com/islet-project/rust-rsi
      A library implementing token and RSI related functionalities (fetching, parsing).
    * rsictl: https://github.com/islet-project/rsictl
      Tool for performing RSI operations from user space.
    * rocli: https://github.com/islet-project/rocli
      Tool for provisioning reference token and CPAK to the Veraison services.
    * ratls: https://github.com/islet-project/ratls
      A library implementing RaTLS protocol for attestation purposes.
    * realm-verifier: https://github.com/islet-project/realm-verifier
      A library for verifying RIM and REMs with reference values.
    * veraison-verifier: https://github.com/islet-project/veraison-verifier
      A library for verifying platform token with reference values using Veraison service.

* veraison: https://github.com/veraison/services

# Preparation

Only the Islet repository should be checked manually:

    CCA/islet

Now run `make` inside the `CCA/islet/examples/veraison` directory. This compiles
some tools that will be used for this demo and places them inside proper
directories. It also copies the `root-ca.crt` used by `realm-application`.

    CCA/islet/examples/veraison $ make

The files installed are:

* `root-ca.crt` copied to `CCA/islet/out/shared`
* `rsictl` installed in `CCA/islet/out/shared/bin` (AARCH64) and
  `CCA/islet/examples/veraison/bin` (X86_64)
* `realm-application` installed in `CCA/islet/out/shared/bin`
* `rocli` installed in `CCA/islet/examples/veraison/bin`
* `reliant-party` installed in `CCA/islet/examples/veraison/bin`

# Provisioning

This is emulated by generating CPAK public key using one of camellia-hes
utilities:

    CCA/islet/hes/cpak-generator $ cargo run

This will by default generate a CPAK using dummy GUK and dummy BL2 hash files
from `CCA/islet/hes/res` directory and save both key binary and PEM format
respectively as:

    CCA/islet/hes/out/cpak_public.bin
    CCA/islet/hes/out/cpak_public.pem

# Gathering measurements

There are 2 things we need to measure here. Platform and realm.

## Platform measurement

The platform measurement is done by getting the whole CCA token. Platform
measurements are saved there.

This is performed by some specifically prepared realm (e.g. one provided by
`CCA/islet/scripts/fvp-cca`). To do this do the following:

    CCA/islet $ ./scripts/init.sh
    CCA/islet $ ./scripts/fvp-cca --normal-world=linux-net --realm=linux --rmm=islet --hes --rmm-log-level info

The first command will initialize the scripts and download all required
components. The second command will build the platform and the realm and run the
FVP emulator and HES application.

If run under X environment terminals should open with telnet 5000/5003. If not
we can run those telnets manually on two separate terminals:

    $ telnet localhost 5000
    $ telnet localhost 5003

Port 5000 is the main terminal with console. 5003 is RMM. We don't need the
output of the second one, but the telnet itself is necessary for FVP to work
properly (buffering reasons).

When the FVP linux is booted we need to run the realm:

    $ ./launch-realm.sh

This will take a lot of time (FVP is slow). Wait until you have a realm
loaded. Then load RSI module and get the token:

    Welcome to Buildroot
    buildroot login: root

    # cd /shared
    shared # insmod rsi.ko
    shared # ./bin/rsictl attest -o token.bin

For the token its challenge value will be randomized, but in here it doesn't
matter. Now we can kill the FVP (ctrl-c on the FVP terminal). Eventually the
following command may be required as FVP doesn't always close cleanly:

    $ pkill -9 -i fvp

The generated token is saved as the following file:

    CCA/islet/out/shared/token.bin

## Realm measurement

Realm measurement is done by generating a json file containing realm information
that will be fed to realm verifier.

### Using kvmtool-rim-measurer (TODO: this needs simplification)

This is performed by a small helper program called `kvmtool-rim-measurer`. It basically
runs a modified lkvm tool that calculates and displays the RIM
value. The process looks as follows:

* generate/get the realm you want to use (for now generated by `fvp-cca` script,
  those files can be taken from `CCA/islet/out/shared`, these files include the linux kernel image `linux.realm`, initrd file `rootfs-realm.cpio.gz` and the shell script used for launching the realm `launch-realm.sh`)
* Build the kvmtool-rim-measurer tool according to the description https://github.com/islet-project/assets/blob/3rd-kvmtool-rim-measurer/BUILD-RIM-MEASURER
* Create a dedicated directory for realm files (e.g. `CCA/islet/out/rim-extractor`) and copy all the realm files we want to measure to that folder
* copy the resulting `lkvm-rim-measurer` to the `CCA/islet/out/rim-extractor` folder
* substitute `lkvm` to `lkvm-rim-measurer` in the `CCA/islet/out/rim-extractor/launch-realm.sh` script
* get into the `CCA/islet/out/rim-extractor` folder and run the `launch-realm.sh` script
* The `lkvm-rim-measurer` will display the resulting RIM (e.g. RIM: F58AF6D6A022F113627B1E0B1E0D9B9A1BFB460207AC29721E84BCEF4B4F5CE08351684444BC11CF329D1D4C807BB621807916C2DF4F56B7326E8D16692546A8)

### Alternatively: extract the RIM from the token file

Display the token using `rsictl` command:

    CCA/islet/examples/veraison $ ./bin/rsictl verify -i ../../out/shared/token.bin | grep 'Realm initial measurement'
    Realm initial measurement      (#44238) = [ace992744cb08283a2c5a31785b2d307a7936825751f9affc64ea37b02d9effb]

RIM value is between `[]` characters.

### Create a refence measurement values file

Create a `reference.json` file using the commands below (replace the
`PASTE_THE_OBTAINED_RIM_HEX_STRING_HERE` with the RIM obtained from one of the
previous steps):

```
export RIM="PASTE_THE_OBTAINED_RIM_HEX_STRING_HERE"

cat > reference.json << EOF
{
    "version": "0.1",
    "issuer": {
        "name": "Samsung",
        "url": "https://cca-realms.samsung.com/"
    },
    "realm": {
        "uuid": "f7e3e8ef-e0cc-4098-98f8-3a12436da040",
        "name": "Data Processing Service",
        "version": "1.0.0",
        "release-timestamp": "2024-09-09T05:21:31Z",
        "attestation-protocol": "HTTPS/RA-TLSv1.0",
        "port": 8088,
        "reference-values": {
            "rim": "$RIM",
            "rems": [
                [
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                    "0000000000000000000000000000000000000000000000000000000000000000"
                ]
            ],
            "hash-algo": "sha-256"
        }
    }
}
EOF
```

The resulting json will be saved as the following file:

    CCA/islet/examples/veraison/reference.json

Caveat: only RIM is supported for now, the REMs are placeholders.

# Provisioning/Measurement summary

Those 2 processes should end with the following things

* Prepared realm that won't be modified anymore:
  `linux.realm rootfs-realm.cpio.gz launch-realm.sh`
  For now we use the one generated by fvp-cca
* Public CPAK key: `cpak_public.bin cpak_public.pem`
* Platform measurement: `token.bin`
* Realm measurement: `reference.json`

CPAK keys, token and measurement files should be _sent_ to verification services
using a _safe_ communication channel.

# Running realm for verification purposes

This is done in almost the same way we run realm to get the token.

Run the FVP with HES and network this time. Use the --run-only param from now on
not to regenerate the realm anymore so our measurements won't get stale:

    CCA/islet $ ./scripts/fvp-cca --normal-world=linux-net --realm=linux --rmm=islet --hes --rmm-log-level info --run-only

When FVP is booted run the realm:

    # ./launch-realm.sh net

Inside the realm you need to do the following:

* configure the network
* load the RSI module
* set the date for the certificates to work properly

This is how it looks:

    Welcome to Buildroot
    buildroot login: root

    # cd /shared
    shared # ./set-realm-ip.sh
    shared # insmod rsi.ko
    shared # date 120512002023

# Running and provisioning verification services (Veraison, realm-verifier)

To bootstrap the Veraison services use the `CCA/islet/examples/veraison/bootstrap.sh`:

    CCA/islet/examples/veraison $ ./bootstrap.sh

In details, it does a couple of things:
* Cloning the `https://github.com/veraison/services` repo to `CCA/islet/exmaples/veraison/services` .
* Applies `./veraison-patch` to fix things and `./veraison-no-auth-patch` to disable endpoints authentication.
* Builds Docker containers by running `CCA/islet/exmaples/veraison/services/deployments/docker/Makefile`.
* Start `Veraison` by running `veraison start`.
* Inserts the `./accept-all.rego` policy which makes `Veraison` accept all implementation IDs etc. (normally you are supposed to create a set of rules checking various parts of the attestation token).

To get access to the `veraison` cli interface source:
* `CCA/islet/exmaples/veraison $ source services/deployments/docker/env.zsh` for zsh,
* `CCA/islet/exmaples/veraison $ source services/deployments/docker/env.bash` for bash.

Check if all 5 veraison services are running:

    $ veraison status
    vts: running
    provisioning: running
    verification: running
    management: running
    keycloak: running

And run provisioning of token and CPAK in PEM format:

    CCA/islet/examples/veraison/provisioning $ ./run.sh -t <path/to/token.bin> -c <path/to/cpak_public.pem>

This will provision a reference token and public CPAK to allow
Veraison verification.

It's possible to see current values stored in Veraison:

    $ veraison stores

Run reliant-party, which is provisioned with `reference.json` and
acts as Reliant Party with communication to realm and Veraison
services (this binary takes several parameters, most should not be of
any concern apart from passing latest reference values in `reference.json`):

    CCA/islet/examples/veraison $ ./bin/reliant-party -r <path/to/reference.json>

If needed, '-b' option can be used to pass different network interface binding
(the default is 0.0.0.0:1337):

    CCA/islet/examples/veraison $ ./bin/reliant-party -r <path/to/reference.json> -b <LOCAL_IP:PORT>

Reliant-party awaits on given IP:PORT for communication from Realm and
utilizes our `ratls` Rust library and `realm-verifier` library (for `reference.json`
reference values verification) to verify client CCA token.

# Verification itself

On the realm side (the one we already run) just trigger the verification
process. This is done using `realm-application` (`CCA/islet/examples/veraison/realm-application`).
It will initialize RATLS connection to verification service by performing the necessary steps:

* receive challenge value from verification service
* request the token from RMM/TF-A/HES using the challenge
* send the received token to verification service
* establish safe connection if verification services agrees to do so

This is done with the following command on the realm:

    shared # ./bin/realm-application -r root-ca.crt -u <SERVER_IP:PORT>

That command will take a very long time as Realm on FVP is slow and it does
asymmetric cryptography (RSA key generation).

# Verification success

When verification succeeds, both `realm-application` and `realm-verifier` should not
output any errors. For both binaries you can set RUST_LOG
environmental variable to change log level (info, debug):

Reliant party:

    CCA/islet/examples/veraison $ RUST_LOG=info ./bin/reliant-party -r <path/to/reference.json> -b <LOCAL_IP:PORT>

Realm:

    shared # RUST_LOG=info ./bin/realm-application -r root-ca.crt -u <SERVER_IP:PORT>

With that log level realm client should report successful socket write
with 'GIT' message and verifying server should output that message.
