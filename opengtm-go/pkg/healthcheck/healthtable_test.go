// Copyright 2025 Allyn L. Bottorff
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package healthcheck

import (
	"net"
	"reflect"
	"testing"
)

func TestBuildFromConfigTCP(t *testing.T) {

	var config = Config{
		TcpPools: []TCPPool{
			{CommonPool{Name: "pool1", Port: 1234, Members: []string{"host1", "host2"}, FallbackIP: "1.1.1.1", Interval: 30}},
			{CommonPool{Name: "pool2", Port: 1234, Members: []string{"host3", "host4"}, FallbackIP: "2.2.2.2", Interval: 30}},
		},
	}

	var table = new(HealthTable)
	table.BuildFromConfig(&config)

	var testTable = new(HealthTable)
	testTable.Health = make(map[string][]Member)
	testTable.Health["pool1"] = []Member{
		{Host: "host1", Ip: net.ParseIP("1.1.1.1"), Healthy: false},
		{Host: "host2", Ip: net.ParseIP("1.1.1.1"), Healthy: false},
	}
	testTable.Health["pool2"] = []Member{
		{Host: "host3", Ip: net.ParseIP("2.2.2.2"), Healthy: false},
		{Host: "host4", Ip: net.ParseIP("2.2.2.2"), Healthy: false},
	}

	if !reflect.DeepEqual(testTable.Health, table.Health) {
		t.Fatalf("%v | %v", testTable.Health, table.Health)
	}

}
