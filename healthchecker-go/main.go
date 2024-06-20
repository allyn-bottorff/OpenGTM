// Copyright 2024 Allyn L. Bottorff
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
	"encoding/json"
	"fmt"
	"os"
)

func main() {

	//Read some config
	data, err := os.ReadFile("./conf.json")
	if err != nil {
		fmt.Println("Failed to read config")
		os.Exit(1)
	}
	var config Config
	err = json.Unmarshal(data, &config)
	if err != nil {
		fmt.Println("Failed to unmarshal config")
		fmt.Println(err)
		os.Exit(1)
	}

	fmt.Printf("%v\n", config)

	errors := config.validate()
	if errors != nil {
		for _, e := range errors {
			fmt.Printf("%v\n", e)
		}
	}

}
