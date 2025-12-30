Name:           elara-launcher
Version:        0.1.0
Release:        1%{?dist}
Summary:        Elara Launcher

License:        MIT
URL:            https://github.com/joshm998/elara-launcher
Source0:        https://github.com/joshm998/elara-launcher/archive/refs/tags/v%{version}.tar.gz

BuildRoot:      %{_tmppath}/%{name}-%{version}-%{release}-root-%(%{__id_u} -n)

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  gtk4-devel
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig(gtk4)

Requires:       gtk4

%description
Elara Launcher is a GTK4-based launcher written in Rust.

%prep
%setup -q -n %{name}-%{version}

%build
make build-release %{?_smp_mflags}

%install
rm -rf %{buildroot}
make install PREFIX=%{_prefix} DESTDIR=%{buildroot}

%files
%defattr(-,root,root,-)
%license LICENSE
%doc README.md
%{_bindir}/elara-launcher

%changelog
* Wed Dec 31 2025 Josh Mangiola <contact@joshmangiola.com> - 0.1.0-1
- Initial release
