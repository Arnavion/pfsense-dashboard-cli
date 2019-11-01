#!/usr/bin/awk -f

BEGIN {
	# Static constants

	interfaces[1] = "em0"
	interfaces[2] = "bridge0"
	interfaces[3] = "igb0"
	interfaces[4] = "igb1"
	interfaces[5] = "igb2"
	interfaces[6] = "igb3"

	services[1, "service_name"] = "dhcpd"; services[1, "process_name"] = "dhcpd"; services[1, "pidfile"] = ""
	services[2, "service_name"] = "dnsbl"; services[2, "process_name"] = "lighttpd_pfb"; services[2, "pidfile"] = ""
	services[3, "service_name"] = "dpinger"; services[3, "process_name"] = "dpinger"; services[3, "pidfile"] = "dpinger_WAN_DHCP~*.pid"
	services[4, "service_name"] = "ntpd"; services[4, "process_name"] = "ntpd"; services[4, "pidfile"] = "ntpd.pid"
	services[5, "service_name"] = "sshd"; services[5, "process_name"] = "sshd"; services[5, "pidfile"] = "sshd.pid"
	services[6, "service_name"] = "syslogd"; services[6, "process_name"] = "syslogd"; services[6, "pidfile"] = "syslog.pid"
	services[7, "service_name"] = "unbound"; services[7, "process_name"] = "unbound"; services[7, "pidfile"] = "unbound.pid"


	# Dynamic constants

	os_version = sprintf( \
		"%s-p%s (%s)",
		read_line("/etc/version"),
		read_line("/etc/version.patch"),
		exec_line("uname -m") \
	)
	os_release_date = read_line("/etc/version.buildtime")
	os_base_version = exec_line("uname -sr")

	split(exec_line("sysctl -n kern.boottime"), boot_time_parts, " ")
	boot_time_secs = boot_time_parts[4]
	boot_time_secs = substr(boot_time_secs, 0, length(boot_time_secs) - 1)
	boot_time_usecs = boot_time_parts[7]
	boot_time = boot_time_secs + boot_time_usecs / 1000000

	physical_memory = exec_line("sysctl -n hw.physmem") + 0
	physical_memory_mib = physical_memory / 1048576
	memory_max = exec_line("sysctl -n vm.stats.vm.v_page_count") + 0

	command = "sysctl -aN | sort"
	thermal_sensors_command = "sysctl -n"
	i = 1
	while ((command | getline) > 0) {
		if (index($0, "temperature") > 0) {
			thermal_sensors[i] = $0
			thermal_sensors_command = sprintf("%s '%s'", thermal_sensors_command, $0)
			i += 1
		}
	}
	close(command)

	split(exec_line("sysctl -n kern.disks"), disk_names, " ")
	for (i in disk_names) {
		disk_name = disk_names[i]
		disks[i, "name"] = disk_name
		disks[i, "info_command"] = sprintf("diskinfo -v '/dev/%s'", disk_name)
		disks[i, "smart_status_command"] = sprintf("smartctl -H '/dev/%s'", disk_name)
	}

	num_services = length(services) / 3
	for (i = 1; i <= num_services; i++) {
		pidfile = services[i, "pidfile"]
		if (pidfile == "") {
			services[i, "status_command"] = sprintf("pgrep -x '%s' >/dev/null; echo $?", services[i, "process_name"])
		}
		else {
			services[i, "status_command"] = sprintf("pgrep -F /var/run/%s -x '%s' >/dev/null 2>/dev/null; echo $?", pidfile, services[i, "process_name"])
		}
	}


	# Dynamic state

	cpu_previous_total = 0
	cpu_previous_idle = 0

	for (i in interfaces) {
		interface_previous_in_bytes[i] = 0
		interface_previous_out_bytes[i] = 0
	}

	previous = get_now()


	# Main loop

	while (1) {
		now = get_now()
		time_since_previous = now - previous


		output = "\x1b[2J\x1b[1;1H\x1b[3J"


		output = output exec_line(sprintf("date -jf '%%s' '%.0f' '+%%Y-%%m-%%d %%H:%%M:%%S'", now))


		output = output "\n"


		output = output sprintf( \
			"\nVersion          : %s",
			os_version \
		)
		output = output sprintf( \
			"\n                   built on %s",
			os_release_date \
		)
		output = output sprintf( \
			"\n                   based on %s",
			os_base_version \
		)


		output = output "\n"


		uptime = now - boot_time
		uptime_days = uptime / (24 * 60 * 60)
		uptime_hours = (uptime % (24 * 60 * 60)) / (60 * 60)
		uptime_mins = (uptime % (60 * 60)) / 60
		uptime_secs = uptime % 60
		output = output sprintf( \
			"\nUptime           : %d days %02d:%02d:%02d",
			uptime_days,
			uptime_hours,
			uptime_mins,
			uptime_secs \
		)


		output = output "\n"


		split(exec_line("sysctl -n kern.cp_time"), cpu_parts, " ")
		cpu_total = 0
		for (i = 1; i <= length(cpu_parts); i++) {
			cpu_total += cpu_parts[i]
		}
		cpu_idle = cpu_parts[5]
		if (cpu_previous_total > 0) {
			cpu_total_diff = cpu_total - cpu_previous_total
			cpu_total_idle = cpu_idle - cpu_previous_idle
			cpu_percent = (cpu_total_diff - cpu_total_idle) * 100 / cpu_total_diff
			output = output sprintf( \
				"\nCPU usage        : %5.1f %%",
				cpu_percent \
			)
		}
		else {
			output = output sprintf("\nCPU usage        :     ? %%")
		}
		cpu_previous_total = cpu_total
		cpu_previous_idle = cpu_idle


		exec_lines("sysctl -n vm.stats.vm.v_inactive_count vm.stats.vm.v_cache_count vm.stats.vm.v_free_count", memory_info)
		memory_inactive = memory_info[1] + 0
		memory_cache = memory_info[2] + 0
		memory_free = memory_info[3] + 0
		memory_used = memory_max - memory_inactive - memory_cache - memory_free
		memory_used_percent = memory_used * 100 / memory_max
		output = output sprintf( \
			"\nMemory usage     : %5.1f %% of %d MiB",
			memory_used_percent,
			physical_memory_mib \
		)


		split(exec_line_match("pfctl -s info", "current entries"), states_parts, " ")
		states_used = states_parts[3] + 0
		states_max = int(physical_memory / 10485760) * 100
		states_used_percent = states_used * 100 / states_max
		output = output sprintf( \
			"\nState table size : %5.1f %% (%7d / %7d)",
			states_used_percent,
			states_used,
			states_max \
		)


		split(exec_line_match("netstat -m", "mbuf clusters in use"), mbufs_parts, " ")
		split(mbufs_parts[1], mbufs_parts_2, "/")
		mbufs_used = mbufs_parts_2[3] + 0
		mbufs_max = mbufs_parts_2[4] + 0
		mbufs_used_percent = mbufs_used * 100 / mbufs_max
		output = output sprintf( \
			"\nMBUF usage       : %5.1f %% (%7d / %7d)",
			mbufs_used_percent,
			mbufs_used,
			mbufs_max \
		)


		output = output sprintf("\nDisk usage       : ")
		command = "df -kt ufs"
		first = 1
		current_line = 0
		while ((command | getline) > 0) {
			current_line += 1
			if (current_line < 2) {
				continue
			}

			mount_point = $6
			filesystem_space_used = $3
			filesystem_space_max = $2
			filesystem_space_used_percent = filesystem_space_used * 100 / filesystem_space_max
			filesystem_space_max_human = human_size_base10(filesystem_space_max * 1024)

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%16s : %6.2f %% of %sB",
				mount_point,
				filesystem_space_used_percent,
				filesystem_space_max_human \
			)
		}
		close(command)


		output = output sprintf("\nSMART status     : ")
		first = 1
		for (i = 1; i <= length(disks) / 3; i++) {
			split(exec_line_match(disks[i, "info_command"], "# Disk ident"), disk_ident_parts, " ")

			split(exec_line_match(disks[i, "smart_status_command"], "SMART overall-health self-assessment test result"), disk_smart_status_parts, ":")

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%10s %15s %s",
				disks[i, "name"],
				disk_ident_parts[1],
				disk_smart_status_parts[2] \
			)
		}


		output = output "\n"


		output = output sprintf("\nTemperatures     : ")
		first = 1
		exec_lines(thermal_sensors_command, thermal_sensors_values)
		for (i = 1; i <= length(thermal_sensors); i++) {
			thermal_sensor_name = thermal_sensors[i]
			thermal_sensor_value = thermal_sensors_values[i]
			sub(/C$/, "", thermal_sensor_value)

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%31s : %5.1f °C",
				thermal_sensor_name,
				thermal_sensor_value \
			)
		}


		output = output "\n"


		output = output sprintf("\nInterfaces       : ")
		first = 1
		for (i = 1; i <= length(interfaces); i++) {
			interface_name = interfaces[i]

			interface_ip = ""
			if (interface_name == "bridge0") {
				interface_status = "active"
			}
			else {
				interface_status = ""
			}
			command = sprintf("ifconfig '%s'", interface_name)
			while ((command | getline) > 0) {
				if (index($0, "inet ") > 0) {
					interface_ip = $2
				}
				else if (index($0, "status:") > 0) {
					interface_status = $2
					for (j = 3; j <= NF; j++) {
						interface_status = sprintf("%s %s", interface_status, $j)
					}
				}

				if (interface_ip != "" && interface_status != "") {
					break
				}
			}
			close(command)

			interface_in_bytes = 0
			interface_out_bytes = 0
			command = sprintf("netstat -I '%s' -bin", interface_name)
			current_line = 0
			while ((command | getline) > 0) {
				current_line += 1
				if (current_line < 2) {
					continue
				}

				interface_in_bytes += $(NF - 4)
				interface_out_bytes += $(NF - 1)
			}
			close(command)

			if (interface_status == "active" && interface_previous_in_bytes[i] > 0 && interface_previous_out_bytes[i] > 0) {
				interface_in_speed = (interface_in_bytes - interface_previous_in_bytes[i]) / time_since_previous
				interface_in_speed_human = human_size_base10(interface_in_speed)

				interface_out_speed = (interface_out_bytes - interface_previous_out_bytes[i]) / time_since_previous
				interface_out_speed_human = human_size_base10(interface_out_speed)
			}
			else {
				interface_in_speed_human = "?  "
				interface_out_speed_human = "?  "
			}

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%7s: %-10s %-15s %8sB/s down %8sB/s up",
				interface_name,
				interface_status,
				interface_ip,
				interface_in_speed_human,
				interface_out_speed_human \
			)

			interface_previous_in_bytes[i] = interface_in_bytes
			interface_previous_out_bytes[i] = interface_out_bytes
		}


		split(exec_line("nc -U /var/run/dpinger_*.sock 2>/dev/null || :"), gateway_pinger_status_parts, " ")
		if (length(gateway_pinger_status_parts) == 4) {
			gateway_latency_average = gateway_pinger_status_parts[2] / 1000
			gateway_latency_stddev = gateway_pinger_status_parts[3] / 1000
			gateway_ping_packet_loss = gateway_pinger_status_parts[4]
			output = output sprintf( \
				"\nGateway ping RTT : average %6.1f ms, stddev %6.1f ms, packet loss %3d %%",
				gateway_latency_average,
				gateway_latency_stddev,
				gateway_ping_packet_loss \
			)
		}
		else {
			output = output "\nGateway ping RTT : dpinger is not running"
		}


		output = output "\n"


		output = output sprintf("\nServices         : ")
		first = 1
		for (i = 1; i <= length(services) / 4; i++) {
			process_running = exec_line(services[i, "status_command"])
			if (process_running == "0") {
				service_status = "running"
			}
			else {
				service_status = "not running"
			}

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%7s: %s",
				services[i, "service_name"],
				service_status \
			)
		}


		output = output "\n"


		output = output sprintf("\nFirewall logs    : ")
		first = 1
		command = "clog /var/log/filter.log | tail -10r"
		while ((command | getline) > 0) {
			split($6, log_line_parts, ",")

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%s %s %s %s %-7s %-3s %4s %15s -> %s:%s",
				$1,
				$2,
				$3,
				log_line_parts[7],
				log_line_parts[5],
				log_line_parts[8],
				log_line_parts[17],
				log_line_parts[19],
				log_line_parts[20],
				log_line_parts[22] \
			)
		}
		close(command)


		printf("%s", output)


		previous = now
		now2 = get_now()
		sleep_time = now + 1 - now2
		if (sleep_time > 0) {
			system(sprintf("sleep '%.3f'", sleep_time))
		}
	}
}

function exec_line(command,      result) {
	command | getline result
	close(command)
	return result
}

function exec_lines(command, result,      i) {
	i = 1
	while ((command | getline) > 0) {
		result[i] = $0
		i += 1
	}
	close(command)
}

function exec_line_match(command, substring,      result) {
	result = ""
	while ((command | getline) > 0) {
		if (index($0, substring) > 0) {
			result = $0
			break
		}
	}
	close(command)
	return result
}

function read_line(filename,      result) {
	getline result < filename
	close(filename)
	return result
}

function get_now() {
	return exec_line("perl -MTime::HiRes=gettimeofday -MPOSIX=strftime -e '($s,$us) = gettimeofday(); printf \"%d.%06d\n\", $s, $us'") + 0
}

function human_size_base10(value) {
	if (value < 1000) {
		return sprintf("%3d    ", value)
	}

	value /= 1000
	if (value < 1000) {
		return sprintf("%5.1f K", value)
	}

	value /= 1000
	if (value < 1000) {
		return sprintf("%5.1f M", value)
	}

	value /= 1000
	if (value < 1000) {
		return sprintf("%5.1f G", value)
	}

	value /= 1000
	return sprintf("%5.1f T", value)
}

function human_size_si(value) {
	if (value < 1024) {
		return sprintf("%4d     ", value)
	}

	value /= 1024
	if (value < 1024) {
		return sprintf("%6.1f Ki", value)
	}

	value /= 1024
	if (value < 1024) {
		return sprintf("%6.1f Mi", value)
	}

	value /= 1024
	return sprintf("%6.1f Gi", value)
}
