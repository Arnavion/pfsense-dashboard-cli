The pfSense status dashboard in your terminal instead of a web browser.


# Example

```
Version          : 2.4.4-RELEASE-p3 (amd64)
                   built on Wed May 15 18:53:44 EDT 2019
                   based on FreeBSD 11.2-RELEASE-p10

Uptime           : 6 days 22:27:49

CPU usage        :   2.8 %
Memory usage     :   1.4 % of 32609 MiB
State table size :   0.0 % (     97 /  326000)
MBUF usage       :   2.1 % (  20756 / 1000000)
Disk usage       :                / :   0.40 % of 241.7 GB
                           /var/run :   2.76 % of   3.5 MB
SMART status     :       ada0  S0Z4NEAC948908  PASSED

Temperatures     :           dev.cpu.0.temperature :  34.0 °C
                             dev.cpu.1.temperature :  30.0 °C
                             dev.cpu.2.temperature :  31.0 °C
                             dev.cpu.3.temperature :  29.0 °C
                   hw.acpi.thermal.tz0.temperature :  27.9 °C
                   hw.acpi.thermal.tz1.temperature :  29.9 °C

Interfaces       :     em0: active     ***.***.***.***  438    B/s down  132    B/s up
                   bridge0: active     192.168.1.1      111    B/s down    8.1 KB/s up
                      igb0: no carrier                      ?  B/s down      ?  B/s up
                      igb1: no carrier                      ?  B/s down      ?  B/s up
                      igb2: active                      179    B/s down  205    B/s up
                      igb3: active                      187    B/s down    2.9 KB/s up
Gateway ping RTT : average   17.9 ms, stddev    1.9 ms, packet loss   0 %

Services         :   dhcpd: running
                     dnsbl: running
                   dpinger: running
                      ntpd: running
                      sshd: running
                   syslogd: running
                   unbound: running

Firewall logs    : Oct 19 23:10:12 block em0 in udp  190.88.192.127 -> ***.***.***.***:9676
                   Oct 19 23:10:11 block em0 in udp  190.88.192.127 -> ***.***.***.***:63251
                   Oct 19 23:10:11 block em0 in udp  190.88.192.127 -> ***.***.***.***:9676
                   Oct 19 23:10:09 block em0 in udp  49.189.181.124 -> ***.***.***.***:9676
                   Oct 19 23:10:08 block em0 in udp  190.88.192.127 -> ***.***.***.***:63251
                   Oct 19 23:10:07 block em0 in udp  98.218.179.181 -> ***.***.***.***:63251
                   Oct 19 23:10:05 block em0 in udp   171.98.127.44 -> ***.***.***.***:9676
                   Oct 19 23:10:04 block em0 in tcp   193.32.161.48 -> ***.***.***.***:8244
                   Oct 19 23:10:01 block em0 in udp  190.88.192.127 -> ***.***.***.***:9676
                   Oct 19 23:10:00 block em0 in tcp  185.234.219.58 -> ***.***.***.***:25
```

The output refreshes every second.


# How to use

1. Edit the `interfaces` array at the top of `pfsense-dashboard.awk` to have the names of the network interfaces you want to monitor, including any bridge interfaces.

   `interfaces[1]` is interpreted as the WAN interface to fetch firewall logs for.

1. Edit the `services` array at the top of `pfsense-dashboard.awk` to have the names and process names of the services you want to monitor.

1. Copy `pfsense-dashboard.awk` to your router.

	```sh
	scp ./pfsense-dashboard.awk root@router:/usr/local/bin/
	```

1. Run `pfsense-dashboard.awk` over ssh.

	```sh
	ssh root@router 'pfsense-dashboard.awk'
	```

The script requires `awk` and `perl`. These are present on a default install.

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
