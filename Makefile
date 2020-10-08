target/release/gluon-update-manager:
	cargo build --release

install: target/release/gluon-update-manager
	install target/release/gluon-update-manager /usr/local/bin/gluon-update-manager
	install gluon-update-manager.example.toml /etc/gluon-update-manager.toml
	install -d /var/lib/gluon-update-manager
	install contrib/gluon-update-manager.service /etc/systemd/system/gluon-update-manager.service

.PHONY: install