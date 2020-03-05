The pfSense status dashboard in your terminal instead of a web browser.


# Example

```
Version          : 2.4.4-RELEASE-p3 (amd64)
                   built on Wed May 15 18:53:44 EDT 2019
                   based on FreeBSD 11.2-RELEASE-p10

Uptime           : 65 days 08:06:57

CPU usage        :   5.6 %
Memory usage     :   1.7 % of 32609 MiB
State table size :   0.0 % (    128 / 3260000)
MBUF usage       :   2.1 % (  21006 / 1000000)
Disk usage       :    / :   0.4 % of 247.5 GB
                   /tmp :   0.1 % of   1.0 GB
                   /var :   2.6 % of   1.0 GB
SMART status     : ada0 S0Z4NEAC948908 PASSED

Temperatures     :           dev.cpu.0.temperature :  33.0 °C
                             dev.cpu.1.temperature :  31.0 °C
                             dev.cpu.2.temperature :  32.0 °C
                             dev.cpu.3.temperature :  29.0 °C
                   hw.acpi.thermal.tz0.temperature :  27.9 °C
                   hw.acpi.thermal.tz1.temperature :  29.9 °C

Interfaces       :     em0: active     ***.***.***.***    4.8 Kb/s down    3.4 Kb/s up
                   bridge0: active     192.168.1.1       25.1 Kb/s down   99.5 Kb/s up
                                       10.10.10.1
                                       fd26:8f92:d8a5:1::1
                      igb0: no carrier                      ?  b/s down      ?  b/s up
                      igb1: no carrier                      ?  b/s down      ?  b/s up
                      igb2: active                        7.8 Kb/s down    3.2 Kb/s up
                      igb3: active                        3.2 Kb/s down   35.0 Kb/s up
Gateway ping RTT : average   18.6 ms, stddev    1.3 ms, packet loss   0 %

Services         :      dhcpd:     running |  pfb_dnsbl:     running |       sshd:     running |
                 |    dpinger:     running | pfb_filter:     running |    syslogd:     running |
                 |       ntpd:     running |      radvd:     running |    unbound:     running |

Firewall logs    : Feb 16 01:03:51 block em0 in   udp  162.219.176.22 -> ***.***.***.***:23828
                   Feb 16 01:03:39 block em0 in   udp   212.92.115.67 -> ***.***.***.***:23181
                   Feb 16 01:03:35 block em0 in   tcp  77.247.108.119 -> ***.***.***.***:5038
                   Feb 16 01:03:23 block em0 in   udp   212.92.115.67 -> ***.***.***.***:23181
                   Feb 16 01:03:22 block em0 in   udp   37.110.94.133 -> ***.***.***.***:23828
                   Feb 16 01:03:21 block em0 in   udp     142.55.3.15 -> ***.***.***.***:19671
                   Feb 16 01:03:16 block em0 in   tcp 122.116.103.221 -> ***.***.***.***:23
                   Feb 16 01:03:13 block em0 in   udp   92.255.228.66 -> ***.***.***.***:23828
                   Feb 16 01:03:08 block em0 in   udp   212.92.115.67 -> ***.***.***.***:23828
                   Feb 16 01:03:05 block em0 in   tcp   92.63.194.148 -> ***.***.***.***:38509
```

The output refreshes every second.


# How to use

1. Edit the `wan_interface` variable, `lan_bridge_interface` variable, and `lan_interfaces` array at the top of `pfsense-dashboard.awk` to have the names of the network interfaces you want to monitor, including any bridge interfaces.

1. Edit the `services` array at the top of `pfsense-dashboard.awk` to have the names and process names of the services you want to monitor.

1. Copy `pfsense-dashboard.awk` to your router.

	```sh
	scp ./pfsense-dashboard.awk root@router:/usr/local/bin/
	```

1. Run `pfsense-dashboard.awk` over ssh.

	```sh
	ssh root@router 'pfsense-dashboard.awk'
	```

The script requires `/usr/bin/awk`, `/usr/local/bin/jq` (`jq`) and `/usr/local/bin/perl` (`perl5`). `awk` is part of base, and `perl5` is pulled in by pfSense, so both are present on a default install. `jq` needs to be installed with `pkg add`, though if you've installed third-party packages like pfBlockerNG then it may have already been pulled in as a dependency.

You need to run the script as `root` if you want to have it show the firewall logs, since the firewall log file is owned by `root:wheel` by default. If you don't need the firewall logs, remove that part of the script, and run it as any limited user with shell access instead.


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
