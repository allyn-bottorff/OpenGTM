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

package main

import (
	"net"
	"testing"
)

func TestTheTests(t *testing.T) {
	// t.Fatal("failed successfully")
}

func TestDumpTable(t *testing.T) {
	var testTable = new(HealthTable)
	testTable.Health = make(map[string][]Member)
	testTable.Health["pool1"] = []Member{
		{Host: "host1", Ip: net.ParseIP("1.1.1.1"), Healthy: false, Cancel: false, Failures: 0},
		{Host: "host2", Ip: net.ParseIP("1.1.1.1"), Healthy: false, Cancel: false, Failures: 0},
	}
	testTable.Health["pool2"] = []Member{
		{Host: "host3", Ip: net.ParseIP("2.2.2.2"), Healthy: false, Cancel: false, Failures: 0},
		{Host: "host4", Ip: net.ParseIP("2.2.2.2"), Healthy: false, Cancel: false, Failures: 0},
	}
	var tableString, err = testTable.dumpTable()
	if err != nil {
		t.Fatal(err)
	}

	var testString = `{"pool1":[{"host":"host1","ip":"1.1.1.1","healthy":false,"cancel":false,"failures":0},{"host":"host2","ip":"1.1.1.1","healthy":false,"cancel":false,"failures":0}],"pool2":[{"host":"host3","ip":"2.2.2.2","healthy":false,"cancel":false,"failures":0},{"host":"host4","ip":"2.2.2.2","healthy":false,"cancel":false,"failures":0}]}`

	if testString != tableString {
		t.Fatalf("Got: %s  Expected: %s", tableString, testString)
	}

}
