#!/usr/bin/awk -f

BEGIN {
	# Static constants

	wan_interface = "em0"
	lan_bridge_interface = "bridge0"
	lan_interfaces[1] = "igb0"
	lan_interfaces[2] = "igb1"
	lan_interfaces[3] = "igb2"
	lan_interfaces[4] = "igb3"

	services[1, "service_name"] = "dhcpd"; services[1, "process_name"] = "dhcpd"; services[1, "pidfile"] = ""
	services[2, "service_name"] = "dpinger"; services[2, "process_name"] = "dpinger"; services[2, "pidfile"] = "dpinger_*.pid"
	services[3, "service_name"] = "ntpd"; services[3, "process_name"] = "ntpd"; services[3, "pidfile"] = "ntpd.pid"
	services[4, "service_name"] = "pfb_dnsbl"; services[4, "process_name"] = "lighttpd_pfb"; services[4, "pidfile"] = ""
	services[5, "service_name"] = "pfb_filter"; services[5, "process_name"] = "php_pfb"; services[5, "pidfile"] = ""
	services[6, "service_name"] = "radvd"; services[6, "process_name"] = "radvd"; services[6, "pidfile"] = "radvd.pid"
	services[7, "service_name"] = "sshd"; services[7, "process_name"] = "sshd"; services[7, "pidfile"] = "sshd.pid"
	services[8, "service_name"] = "syslogd"; services[8, "process_name"] = "syslogd"; services[8, "pidfile"] = "syslog.pid"
	services[9, "service_name"] = "unbound"; services[9, "process_name"] = "unbound"; services[9, "pidfile"] = "unbound.pid"

	netstat_command = " \
		netstat -bin --libxo json | \
		jq ' \
			reduce .statistics.interface[] as { name: $name, network: $network, address: $address, \"received-bytes\": $in, \"sent-bytes\": $out } \
			({}; .[$name] = { \
				addresses: (.[$name].addresses + ( \
					if ( \
						($network | startswith(\"<Link\")) or \
						($network | startswith(\"<Link\")) or \
						($address | startswith(\"fe80:\")) \
					) then [] else [$address] end \
				)), \
				in: (.[$name].in + $in), \
				out: (.[$name].out + $out) \
			}) | \
			to_entries[] | \
			\"\\(.key) \\(.value.in) \\(.value.out) \\(.value.addresses | join(\" \"))\" \
		' -r \
	"


	# Dynamic constants

	os_version = sprintf( \
		"%s-p%s (%s)",
		read_line("/etc/version"),
		read_line("/etc/version.patch"),
		exec_line("uname -m") \
	)
	os_release_date = read_line("/etc/version.buildtime")
	os_base_version = exec_line("uname -sr")

	split(exec_line("sysctl -b kern.boottime | od -t uI"), boot_time_parts, " ")
	boot_time_secs = (boot_time_parts[3] * 2 ^ 32) + boot_time_parts[2]
	boot_time_usecs = (boot_time_parts[5] * 2 ^ 32) + boot_time_parts[4]
	boot_time = boot_time_secs + boot_time_usecs / 1000000

	physical_memory = exec_line("sysctl -n hw.physmem") + 0
	physical_memory_mib = physical_memory / 1048576
	memory_max = exec_line("sysctl -n vm.stats.vm.v_page_count") + 0

	command = "df -kt ufs"
	max_mount_point_len = 0
	current_line = 0
	while ((command | getline) > 0) {
		current_line += 1
		if (current_line < 2) {
			continue
		}

		mount_point_len = length($6)
		if (mount_point_len > max_mount_point_len) {
			max_mount_point_len = mount_point_len
		}
	}
	close(command)
	disk_usage_format = sprintf("\x1B[%%sm%%%ds : %%5.1f %%%% of %%sB\x1B[0m", max_mount_point_len)

	max_disk_name_len = 0
	max_disk_serial_number_len = 0
	split(exec_line("sysctl -n kern.disks"), disk_names, " ")
	for (i in disk_names) {
		disk_name = disk_names[i]
		disks[i, "name"] = disk_name

		disks[i, "serial_number"] = exec_line(sprintf("smartctl -ij '/dev/%s' | jq '.serial_number' -r", disk_name))

		disks[i, "smart_status_command"] = sprintf("smartctl -Hj '/dev/%s' | jq '.smart_status.passed'", disk_name)

		disk_name_len = length(disk_name)
		if (disk_name_len > max_disk_name_len) {
			max_disk_name_len = disk_name_len
		}

		disk_serial_number_len = length(disk_serial_number)
		if (disk_serial_number_len > max_disk_serial_number_len) {
			max_disk_serial_number_len = disk_serial_number_len
		}
	}
	disk_status_format = sprintf("\x1B[%%sm%%%ds %%%ds %%s\x1B[0m", max_disk_name_len, max_disk_serial_number_len)
	num_disks = length(disks) / 3

	max_thermal_sensor_name_len = 0
	command = "sysctl -aN | sort"
	thermal_sensors_command = "sysctl -b"
	i = 1
	while ((command | getline) > 0) {
		if (index($0, "temperature") > 0) {
			thermal_sensors[i] = $0
			thermal_sensors_command = sprintf("%s '%s'", thermal_sensors_command, $0)

			thermal_sensor_name_len = length($0)
			if (thermal_sensor_name_len > max_thermal_sensor_name_len) {
				max_thermal_sensor_name_len = thermal_sensor_name_len
			}

			i += 1
		}
	}
	thermal_sensors_command = thermal_sensors_command " | hexdump -v -e '\"%u \"'"
	close(command)
	thermal_sensor_format = sprintf("\x1B[%%sm%%%ds : %%5.1f °C\x1B[0m", max_thermal_sensor_name_len)

	interfaces[1, "name"] = wan_interface
	interfaces[1, "ifconfig_command"] = sprintf("ifconfig '%s'", wan_interface)
	interfaces[2, "name"] = lan_bridge_interface
	interfaces[2, "ifconfig_command"] = sprintf("ifconfig '%s'", lan_bridge_interface)
	for (i in lan_interfaces) {
		interface_name = lan_interfaces[i]
		interfaces[i + 2, "name"] = interface_name
		interfaces[i + 2, "ifconfig_command"] = sprintf("ifconfig '%s'", interface_name)
	}
	num_interfaces = length(interfaces) / 2

	max_interface_name_len = 0
	for (i = 1; i <= num_interfaces; i++) {
		interface_name_len = length(interfaces[i, "name"])
		if (interface_name_len > max_interface_name_len) {
			max_interface_name_len = interface_name_len
		}
	}
	interface_status_format1 = sprintf("\x1B[%%sm%%%ds: %%-10s %%-15s %%8sb/s down %%8sb/s up\x1B[0m", max_interface_name_len)
	interface_status_format2 = sprintf("\n\x1B[%%sm                   %%%ds             %%s\x1B[0m", max_interface_name_len)

	max_service_name_len = 0
	num_services = length(services) / 3
	for (i = 1; i <= num_services; i++) {
		pidfile = services[i, "pidfile"]
		if (pidfile == "") {
			services[i, "status_command"] = sprintf("pgrep -x '%s' >/dev/null; echo $?", services[i, "process_name"])
		}
		else {
			services[i, "status_command"] = sprintf("pgrep -F /var/run/%s -x '%s' >/dev/null 2>/dev/null; echo $?", pidfile, services[i, "process_name"])
		}

		service_name_len = length(services[i, "service_name"])
		if (service_name_len > max_service_name_len) {
			max_service_name_len = service_name_len
		}
	}
	service_status_format = sprintf(" \x1B[%%sm%%-%ds\x1B[0m |", max_service_name_len)
	num_services = length(services) / 4
	num_services_per_row = int(80 / (max_service_name_len + 3))
	num_services_rows = int((num_services + num_services_per_row - 1) / num_services_per_row)

	firewall_logs_command = sprintf( \
		"clog /var/log/filter.log | grep '%s' | tail -10r",
		wan_interface \
	)


	# Dynamic state

	cpu_previous_total = 0
	cpu_previous_idle = 0

	for (i = 1; i <= num_interfaces; i++) {
		interface_previous_in_bytes[i] = 0
		interface_previous_out_bytes[i] = 0
	}

	previous = get_now()


	# Main loop

	while (1) {
		now = get_now()
		time_since_previous = now - previous


		output = "\x1B[2J\x1B[1;1H\x1B[3J"


		output = output sprintf( \
			"Version          : %s",
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
			cpu_usage_color = get_color_for_usage(cpu_percent)
			output = output sprintf( \
				"\nCPU usage        : \x1B[%sm%5.1f %%\x1B[0m",
				cpu_usage_color,
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
		memory_usage_color = get_color_for_usage(memory_used_percent)
		output = output sprintf( \
			"\nMemory usage     : \x1B[%sm%5.1f %% of %d MiB\x1B[0m",
			memory_usage_color,
			memory_used_percent,
			physical_memory_mib \
		)


		split(exec_line_match("pfctl -s info", "current entries"), states_parts, " ")
		states_used = states_parts[3] + 0
		states_max = int(physical_memory / 10485760) * 1000
		states_used_percent = states_used * 100 / states_max
		states_usage_color = get_color_for_usage(states_used_percent)
		output = output sprintf( \
			"\nState table size : \x1B[%sm%5.1f %% (%7d / %7d)\x1B[0m",
			states_usage_color,
			states_used_percent,
			states_used,
			states_max \
		)


		split(exec_line_match("netstat -m", "mbuf clusters in use"), mbufs_parts, " ")
		split(mbufs_parts[1], mbufs_parts_2, "/")
		mbufs_used = mbufs_parts_2[3] + 0
		mbufs_max = mbufs_parts_2[4] + 0
		mbufs_used_percent = mbufs_used * 100 / mbufs_max
		mbufs_usage_color = get_color_for_usage(mbufs_used_percent)
		output = output sprintf( \
			"\nMBUF usage       : \x1B[%sm%5.1f %% (%7d / %7d)\x1B[0m",
			mbufs_usage_color,
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
			filesystem_space_usage_color = get_color_for_usage(filesystem_space_used_percent)
			filesystem_space_max_human = human_size_base10(filesystem_space_max * 1024)

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				disk_usage_format,
				filesystem_space_usage_color,
				mount_point,
				filesystem_space_used_percent,
				filesystem_space_max_human \
			)
		}
		close(command)


		output = output sprintf("\nSMART status     : ")
		first = 1
		for (i = 1; i <= num_disks; i++) {
			disk_smart_passed = exec_line(disks[i, "smart_status_command"])
			if (disk_smart_passed == "true") {
				disk_smart_status = "PASSED"
				disk_smart_color = get_color_for_up_down(1)
			}
			else {
				disk_smart_status = "FAILED"
				disk_smart_color = get_color_for_up_down(0)
			}

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				disk_status_format,
				disk_smart_color,
				disks[i, "name"],
				disks[i, "serial_number"],
				disk_smart_status \
			)
		}


		output = output "\n"


		output = output sprintf("\nTemperatures     : ")
		first = 1
		split(exec_line(thermal_sensors_command), thermal_sensors_values, " ")
		for (i = 1; i <= length(thermal_sensors); i++) {
			thermal_sensor_name = thermal_sensors[i]
			thermal_sensor_value = thermal_sensors_values[i] / 10 - 273.15
			thermal_sensor_color = get_color_for_temperature(thermal_sensor_value)

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				thermal_sensor_format,
				thermal_sensor_color,
				thermal_sensor_name,
				thermal_sensor_value \
			)
		}


		output = output "\n"


		output = output sprintf("\nInterfaces       : ")

		command = netstat_command
		while ((command | getline) > 0) {
			netstat_output[$1] = $0
		}
		close(command)

		first = 1
		for (i = 1; i <= num_interfaces; i++) {
			interface_name = interfaces[i, "name"]

			if (interface_name == lan_bridge_interface) {
				interface_status = "active"
			}
			else {
				interface_status = ""
			}
			interface_status_output = exec_line_match(interfaces[i, "ifconfig_command"], "status:")
			if (interface_status_output != "") {
				split(interface_status_output, interface_status_parts, " ")
				interface_status_parts_length = length(interface_status_parts)
				interface_status = interface_status_parts[2]
				for (j = 3; j <= interface_status_parts_length; j++) {
					interface_status = sprintf("%s %s", interface_status, interface_status_parts[j])
				}
			}

			interface_netstat_output = netstat_output[interfaces[i, "name"]]
			split(interface_netstat_output, interface_netstat_info, " ")
			interface_netstat_info_length = length(interface_netstat_info)

			interface_in_bytes = interface_netstat_info[2]
			interface_out_bytes = interface_netstat_info[3]

			interface_ip = ""
			num_interface_other_ips = 0
			for (j = 4; j <= interface_netstat_info_length; j++) {
				one_interface_ip = interface_netstat_info[j]
				if (interface_ip == "" && length(one_interface_ip) <= 15) {
					interface_ip = one_interface_ip
				}
				else {
					num_interface_other_ips += 1
					interface_other_ips[num_interface_other_ips] = one_interface_ip
				}
			}

			if (interface_status == "active" && interface_previous_in_bytes[i] > 0 && interface_previous_out_bytes[i] > 0) {
				interface_in_speed = (interface_in_bytes - interface_previous_in_bytes[i]) / time_since_previous * 8
				interface_in_speed_human = human_size_base10(interface_in_speed)

				interface_out_speed = (interface_out_bytes - interface_previous_out_bytes[i]) / time_since_previous * 8
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

			interface_status_color = get_color_for_up_down(interface_status == "active")

			output = output sprintf( \
				interface_status_format1,
				interface_status_color,
				interface_name,
				interface_status,
				interface_ip,
				interface_in_speed_human,
				interface_out_speed_human \
			)

			for (j = 1; j <= num_interface_other_ips; j++) {
				output = output sprintf( \
					interface_status_format2,
					interface_status_color,
					"",
					interface_other_ips[j] \
				)
			}

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


		output = output sprintf("\nServices         :")
		for (i = 1; i <= num_services_rows; i++) {
			for (j = 1; j <= num_services_per_row; j++) {
				service_index = i + num_services_rows * (j - 1)
				if (service_index > num_services) {
					break
				}

				process_running = exec_line(services[service_index, "status_command"])
				if (process_running == "0") {
					service_color = get_color_for_up_down(1)
				}
				else {
					service_color = get_color_for_up_down(0)
				}

				if (i > 1 && j == 1) {
					output = output sprintf("\n                 |")
				}

				output = output sprintf( \
					service_status_format,
					service_color,
					services[service_index, "service_name"] \
				)
			}
		}


		output = output "\n"


		output = output sprintf("\nFirewall logs    : ")
		first = 1
		while ((firewall_logs_command | getline) > 0) {
			split($6, log_line_parts, ",")

			if (first) {
				first = 0
			}
			else {
				output = output sprintf("\n                   ")
			}

			output = output sprintf( \
				"%s %02s %s %s %s %-3s %4s %15s -> %s:%s",
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
		close(firewall_logs_command)


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

function get_color_for_usage(usage) {
	if (usage < 5) {
		return "0;34";
	}
	if (usage < 10) {
		return "1;34";
	}
	if (usage < 25) {
		return "1;32";
	}
	if (usage < 50) {
		return "1;33";
	}
	if (usage < 75) {
		return "0;33";
	}
	if (usage < 90) {
		return "1;31";
	}
	return "0;31";
}

function get_color_for_temperature(temp) {
	if (temp < 30) {
		return "0;34";
	}
	if (temp < 35) {
		return "1;34";
	}
	if (temp < 40) {
		return "1;32";
	}
	if (temp < 45) {
		return "1;33";
	}
	if (temp < 55) {
		return "0;33";
	}
	if (temp < 65) {
		return "1;31";
	}
	return "0;31";
}

function get_color_for_up_down(is_up) {
	if (is_up) {
		return "1;32";
	}
	else {
		return "0;31";
	}
}
