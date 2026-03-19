sudo cp 99-switchboard.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
