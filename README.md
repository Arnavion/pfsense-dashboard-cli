The pfSense status dashboard in your terminal instead of a web browser.


# Example

```
Version          : 2.4.5-RELEASE (amd64)
                   built on Tue Mar 24 15:25:50 EDT 2020
                   based on FreeBSD 11.3-STABLE

Uptime           : 3 days 00:53:03

CPU usage        :   4.9 %
Memory usage     :   8.4 % of 32609 MiB
State table size :   0.0 % (    211 / 3260000)
MBUF usage       :   2.0 % (  20496 / 1000000)
Disk usage       :    / :   0.5 % of 247.5 GB
                   /tmp :   0.1 % of   1.0 GB
                   /var :   2.6 % of   1.0 GB
SMART status     : ada0 S0Z4NEAC948908 PASSED

Temperatures     :           dev.cpu.0.temperature :  29.0 °C
                             dev.cpu.1.temperature :  31.0 °C
                             dev.cpu.2.temperature :  29.0 °C
                             dev.cpu.3.temperature :  26.0 °C
                   hw.acpi.thermal.tz0.temperature :  27.9 °C
                   hw.acpi.thermal.tz1.temperature :  29.9 °C
                                              ada0 :  27.0 °C

Interfaces       :     em0 :   1.3 Kb/s down   1.8 Kb/s up ***.***.***.***
                      gif0 :   0    b/s down   0    b/s up ****:***:****:****::*
                   bridge0 :                               ****:***:****:****::1
                                                           10.10.10.1
                                                           192.168.1.1
                      igb0 : no carrier
                      igb1 : no carrier
                      igb2 :   13.3 Kb/s down    5.1 Kb/s up
                      igb3 :  141.5 Kb/s down  376.0 Kb/s up
Gateways         :  em0 :   19.7 ms (   1.8 ms),   0 %
                   gif0 :   19.7 ms (   0.5 ms),   0 %

Services         : dhcpd       pfb_dnsbl   radvd       syslogd
                   ntpd        pfb_filter  sshd        unbound

Firewall logs    : Mar 29 15:27:24 em0  block 26063/udp <- 186.79.169.243
                   Mar 29 15:27:14 em0  block 20168/tcp <- 194.26.29.129
                   Mar 29 15:27:08 em0  block  3399/tcp <- 50.227.144.229
                   Mar 29 15:27:08 em0  block 14604/tcp <- 5.135.253.172
                   Mar 29 15:26:58 em0  block 57275/tcp <- 92.118.37.74
                   Mar 29 15:26:56 em0  block 24369/tcp <- 167.99.203.202
                   Mar 29 15:26:43 em0  block 37731/tcp <- 185.176.27.174
                   Mar 29 15:26:40 em0  block  2288/tcp <- 92.63.196.6
                   Mar 29 15:26:38 em0  block  1243/tcp <- 80.82.78.20
                   Mar 29 15:26:35 em0  block 24629/tcp <- 185.176.27.58
```

The output refreshes every second. It also uses colors that are not visible here.


# How to use

1. Copy config.yaml.example to `~/.config/pfsense-dashboard/config.yaml` and edit it to match your router.

1. Build and install the binary under `$PATH`, such as in `~/bin`.

   ```sh
   cargo build --release
   cp -f ./target/release/pfsense-dashboard ~/bin/
   ```

   `make install` will do this for you.

1. Run `pfsense-dashboard`.

   ```sh
   pfsense-dashboard
   ```

Note, the program assumes your router uses a little-endian x86_64 C ABI. If this is not the case, edit the constants in the "Router C ABI definitions" section at the top of `src/main.rs`.


# AWK version

For the older `awk` script version of the dashboard that ran on the router, see <https://github.com/Arnavion/pfsense-dashboard-cli/tree/9ee00b89a20fd88aaede4d53d36100fbe68f1439>


# License

```
pfsense-dashboard-cli

https://github.com/Arnavion/pfsense-dashboard-cli

Copyright 2019 Arnav Singh

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
